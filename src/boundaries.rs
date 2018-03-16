extern crate geo;
extern crate osmpbfreader;

use std::collections::BTreeMap;
use geo::{Coordinate, LineString, MultiPolygon, Point, Polygon};

#[cfg(test)]
use osm_builder::named_node;
#[cfg(test)]
use osm_builder;

struct BoundaryPart {
    nodes: Vec<osmpbfreader::Node>,
}

impl BoundaryPart {
    pub fn new(nodes: Vec<osmpbfreader::Node>) -> BoundaryPart {
        BoundaryPart { nodes: nodes }
    }
    pub fn first(&self) -> osmpbfreader::NodeId {
        self.nodes.first().unwrap().id
    }
    pub fn last(&self) -> osmpbfreader::NodeId {
        self.nodes.last().unwrap().id
    }
}

fn get_nodes(
    way: &osmpbfreader::Way,
    objects: &BTreeMap<osmpbfreader::OsmId, osmpbfreader::OsmObj>,
) -> Vec<osmpbfreader::Node> {
    way.nodes
        .iter()
        .filter_map(|node_id| objects.get(&osmpbfreader::OsmId::Node(*node_id)))
        .filter_map(|node_obj| {
            if let osmpbfreader::OsmObj::Node(ref node) = *node_obj {
                Some(node.clone())
            } else {
                None
            }
        })
        .collect()
}

#[test]
fn test_get_nodes() {
    let mut objects = BTreeMap::new();
    let way = osmpbfreader::Way {
        id: osmpbfreader::WayId(12),
        nodes: [12, 15, 8, 68]
            .iter()
            .map(|&id| osmpbfreader::NodeId(id))
            .collect(),
        tags: osmpbfreader::Tags::new(),
    };
    objects.insert(way.id.into(), way.clone().into());
    let node_12 = osmpbfreader::Node {
        id: osmpbfreader::NodeId(12),
        decimicro_lat: 12000000,
        decimicro_lon: 37000000,
        tags: osmpbfreader::Tags::new(),
    };
    objects.insert(node_12.id.into(), node_12.into());
    let node_13 = osmpbfreader::Node {
        id: osmpbfreader::NodeId(13),
        decimicro_lat: 15000000,
        decimicro_lon: 35000000,
        tags: osmpbfreader::Tags::new(),
    };
    objects.insert(node_13.id.into(), node_13.into());
    let node_15 = osmpbfreader::Node {
        id: osmpbfreader::NodeId(15),
        decimicro_lat: 75000000,
        decimicro_lon: 135000000,
        tags: osmpbfreader::Tags::new(),
    };
    objects.insert(node_15.id.into(), node_15.into());
    let node_8 = osmpbfreader::Node {
        id: osmpbfreader::NodeId(8),
        decimicro_lat: 55000000,
        decimicro_lon: 635000000,
        tags: osmpbfreader::Tags::new(),
    };
    objects.insert(node_8.id.into(), node_8.into());
    let node_68 = osmpbfreader::Node {
        id: osmpbfreader::NodeId(68),
        decimicro_lat: 455000000,
        decimicro_lon: 535000000,
        tags: osmpbfreader::Tags::new(),
    };
    objects.insert(node_68.id.into(), node_68.into());

    let nodes = get_nodes(&way, &objects);
    assert_eq!(nodes.len(), 4);
    assert_eq!(nodes[0].id.0, 12);
    assert_eq!(nodes[1].id.0, 15);
    assert_eq!(nodes[2].id.0, 8);
    assert_eq!(nodes[3].id.0, 68);
}

pub fn build_boundary(
    relation: &osmpbfreader::Relation,
    objects: &BTreeMap<osmpbfreader::OsmId, osmpbfreader::OsmObj>,
) -> Option<MultiPolygon<f64>> {
    use geo::algorithm::contains::Contains;

    let outer_polys = build_boundary_parts(relation, objects, vec!["outer", "enclave", ""]);
    let inner_polys = build_boundary_parts(relation, objects, vec!["inner"]);
    let mut multipoly = MultiPolygon(vec![]);

    if inner_polys.is_none() {
        return outer_polys;
    }
    let mut inner_polys = inner_polys.unwrap();

    if outer_polys.is_some() {
        let outer_polys = outer_polys.unwrap();
        for outer in outer_polys.0 {
            let mut added_outer = false;
            let mut i = 0;
            let mut inners = vec![];
            while i != inner_polys.0.len() {
                let current_inner = inner_polys.0.get(i).unwrap().clone();
                if outer.contains(&current_inner) {
                    inner_polys.0.remove(i);
                    inners.push(current_inner.exterior);
                    added_outer = true;
                } else {
                    i += 1;
                };
            }
            if !added_outer {
                multipoly.0.push(outer.clone());
            } else {
                multipoly
                    .0
                    .push(Polygon::new(outer.exterior.clone(), inners));
            }
        }
    }
    if multipoly.0.is_empty() {
        None
    } else {
        Some(multipoly)
    }
}

pub fn build_boundary_parts(
    relation: &osmpbfreader::Relation,
    objects: &BTreeMap<osmpbfreader::OsmId, osmpbfreader::OsmObj>,
    roles_to_extact: Vec<&str>,
) -> Option<MultiPolygon<f64>> {
    let roles = roles_to_extact;
    let mut boundary_parts: Vec<BoundaryPart> = relation
        .refs
        .iter()
        .filter(|r| roles.contains(&r.role.as_str()))
        .filter_map(|r| {
            let obj = objects.get(&r.member);
            if obj.is_none() {
                debug!(
                    "missing element {:?} for relation {}",
                    r.member, relation.id.0
                );
            }
            obj
        })
        .filter_map(|way_obj| way_obj.way())
        .map(|way| get_nodes(way, objects))
        .filter(|nodes| nodes.len() > 1)
        .map(BoundaryPart::new)
        .collect();
    let mut multipoly = MultiPolygon(vec![]);
    // we want to try to build a polygon for a least each way
    while !boundary_parts.is_empty() {
        let first_part = boundary_parts.remove(0);
        let first = first_part.first();
        let mut current = first_part.last();
        let mut poly_geom = first_part.nodes;

        loop {
            let mut added_part = false;
            let mut i = 0;
            while i < boundary_parts.len() && current != first {
                if current == boundary_parts[i].first() {
                    // the start of current way touch the polygon, we add it and remove it from the
                    // pool
                    current = boundary_parts[i].last();
                    poly_geom.append(&mut boundary_parts[i].nodes);
                    boundary_parts.remove(i);
                    added_part = true;
                } else if current == boundary_parts[i].last() {
                    // the end of the current way touch the polygon, we reverse the way and add it
                    current = boundary_parts[i].first();
                    boundary_parts[i].nodes.reverse();
                    poly_geom.append(&mut boundary_parts[i].nodes);
                    boundary_parts.remove(i);
                    added_part = true;
                } else {
                    i += 1;
                    // didn't do anything, we want to explore the next way, if we had do something we
                    // will have removed the current way and there will be no need to increment
                }
            }
            if current == first {
                // our polygon is closed, we create it and add it to the multipolygon
                let poly_geom = poly_geom
                    .iter()
                    .map(|n| {
                        Point(Coordinate {
                            x: n.lon(),
                            y: n.lat(),
                        })
                    })
                    .collect();
                multipoly
                    .0
                    .push(Polygon::new(LineString(poly_geom), vec![]));
                break;
            }
            if !added_part {
                use geo::haversine_distance::HaversineDistance;
                let p = |n: &osmpbfreader::Node| {
                    Point(Coordinate {
                        x: n.lon(),
                        y: n.lat(),
                    })
                };
                let distance =
                    p(poly_geom.first().unwrap()).haversine_distance(&p(poly_geom.last().unwrap()));
                if distance < 10. {
                    warn!(
                        "boundary: relation/{} ({}): unclosed polygon, dist({:?}, {:?}) = {}",
                        relation.id.0,
                        relation.tags.get("name").map_or("", String::as_str),
                        poly_geom.first().unwrap().id,
                        poly_geom.last().unwrap().id,
                        distance
                    );
                }
                break;
            }
        }
    }
    if multipoly.0.is_empty() {
        None
    } else {
        Some(multipoly)
    }
}

#[test]
fn test_build_boundary_empty() {
    let objects = BTreeMap::new();
    let mut relation = osmpbfreader::Relation {
        id: osmpbfreader::RelationId(12),
        refs: vec![],
        tags: osmpbfreader::Tags::new(),
    };
    relation.refs.push(osmpbfreader::Ref {
        member: osmpbfreader::WayId(4).into(),
        role: "outer".to_string(),
    });
    relation.refs.push(osmpbfreader::Ref {
        member: osmpbfreader::WayId(65).into(),
        role: "outer".to_string(),
    });
    relation.refs.push(osmpbfreader::Ref {
        member: osmpbfreader::WayId(22).into(),
        role: "".to_string(),
    });
    assert!(build_boundary(&relation, &objects).is_none());
}

#[test]
fn test_build_boundary_not_closed() {
    let mut builder = osm_builder::OsmBuilder::new();
    let rel_id = builder
        .relation()
        .outer(vec![
            named_node(3.4, 5.2, "start"),
            named_node(5.4, 5.1, "1"),
        ])
        .outer(vec![named_node(5.4, 5.1, "1"), named_node(2.4, 3.1, "2")])
        .outer(vec![named_node(2.4, 3.2, "2"), named_node(6.4, 6.1, "end")])
        .relation_id
        .into();
    if let osmpbfreader::OsmObj::Relation(ref relation) = builder.objects[&rel_id] {
        assert!(build_boundary(&relation, &builder.objects).is_none());
    } else {
        assert!(false); //this should not happen
    }
}

#[test]
fn test_build_boundary_closed() {
    let mut builder = osm_builder::OsmBuilder::new();
    let rel_id = builder
        .relation()
        .outer(vec![
            named_node(3.4, 5.2, "start"),
            named_node(5.4, 5.1, "1"),
        ])
        .outer(vec![named_node(5.4, 5.1, "1"), named_node(2.4, 3.1, "2")])
        .outer(vec![
            named_node(2.4, 3.2, "2"),
            named_node(6.4, 6.1, "start"),
        ])
        .relation_id
        .into();
    if let osmpbfreader::OsmObj::Relation(ref relation) = builder.objects[&rel_id] {
        let multipolygon = build_boundary(&relation, &builder.objects);
        assert!(multipolygon.is_some());
        let multipolygon = multipolygon.unwrap();
        assert_eq!(multipolygon.0.len(), 1);
    } else {
        assert!(false); //this should not happen
    }
}

#[test]
fn test_build_boundary_closed_reverse() {
    let mut builder = osm_builder::OsmBuilder::new();
    let rel_id = builder
        .relation()
        .outer(vec![
            named_node(2.4, 3.2, "2"),
            named_node(6.4, 6.1, "start"),
        ])
        .outer(vec![named_node(5.4, 5.1, "1"), named_node(2.4, 3.1, "2")])
        .outer(vec![
            named_node(3.4, 5.2, "start"),
            named_node(5.4, 5.1, "1"),
        ])
        .relation_id
        .into();
    if let osmpbfreader::OsmObj::Relation(ref relation) = builder.objects[&rel_id] {
        let multipolygon = build_boundary(&relation, &builder.objects);
        assert!(multipolygon.is_some());
        let multipolygon = multipolygon.unwrap();
        assert_eq!(multipolygon.0.len(), 1);
    } else {
        assert!(false); //this should not happen
    }
}

#[test]
fn test_build_one_boundary_closed() {
    let mut builder = osm_builder::OsmBuilder::new();
    let rel_id = builder
        .relation()
        .outer(vec![
            named_node(3.4, 5.2, "start"),
            named_node(5.4, 5.1, "1"),
            named_node(2.4, 3.1, "2"),
            named_node(6.4, 6.1, "start"),
        ])
        .relation_id
        .into();
    if let osmpbfreader::OsmObj::Relation(ref relation) = builder.objects[&rel_id] {
        let multipolygon = build_boundary(&relation, &builder.objects);
        assert!(multipolygon.is_some());
        let multipolygon = multipolygon.unwrap();
        assert_eq!(multipolygon.0.len(), 1);
    } else {
        assert!(false); //this should not happen
    }
}

#[test]
fn test_build_two_opposite_clockwise_boundaries() {
    use geo::algorithm::centroid::Centroid;
    let mut builder = osm_builder::OsmBuilder::new();
    let rel_id = builder
        .relation()
        .outer(vec![
            named_node(0.0, 0.0, "start"), // anti-clockwise polygon
            named_node(0.0, 1.0, "1"),
            named_node(1.0, 1.0, "2"),
            named_node(1.0, 0.0, "3"),
            named_node(0.0, 0.0, "start"),
        ])
        .outer(vec![
            named_node(0.0, 0.0, "another_start"), // clockwise polygon
            named_node(0.0, -1.0, "4"),
            named_node(-1.0, -1.0, "5"),
            named_node(-1.0, 0.0, "6"),
            named_node(0.0, 0.0, "another_start"),
        ])
        .relation_id
        .into();
    if let osmpbfreader::OsmObj::Relation(ref relation) = builder.objects[&rel_id] {
        let multipolygon = build_boundary(&relation, &builder.objects);
        assert!(multipolygon.is_some());
        let multipolygon = multipolygon.unwrap();
        assert_eq!(multipolygon.0.len(), 2);
        let centroid = multipolygon.centroid();
        let centroid = centroid.unwrap();
        assert_eq!(centroid.lng(), 0.0);
        assert_eq!(centroid.lat(), 0.0);
    } else {
        assert!(false); //this should not happen
    }
}

#[test]
fn test_build_two_boundaries_closed() {
    let mut builder = osm_builder::OsmBuilder::new();
    let rel_id = builder
        .relation()
        .outer(vec![
            named_node(3.4, 5.2, "start"),
            named_node(5.4, 5.1, "1"),
            named_node(2.4, 3.1, "2"),
            named_node(6.4, 6.1, "start"),
        ])
        .outer(vec![
            named_node(13.4, 15.2, "1start"),
            named_node(15.4, 15.1, "11"),
            named_node(12.4, 13.1, "12"),
            named_node(16.4, 16.1, "1start"),
        ])
        .relation_id
        .into();
    if let osmpbfreader::OsmObj::Relation(ref relation) = builder.objects[&rel_id] {
        let multipolygon = build_boundary(&relation, &builder.objects);
        assert!(multipolygon.is_some());
        let multipolygon = multipolygon.unwrap();
        assert_eq!(multipolygon.0.len(), 2);
    } else {
        assert!(false); //this should not happen
    }
}

#[test]
fn test_build_one_donut_boundary() {
    use geo::algorithm::area::Area;
    let mut builder = osm_builder::OsmBuilder::new();
    let rel_id = builder
        .relation()
        .outer(vec![
            named_node(0.0, 0.0, "start"),
            named_node(4.0, 0.0, "1"),
            named_node(4.0, 4.0, "2"),
            named_node(0.0, 4.0, "3"),
            named_node(0.0, 0.0, "start"),
        ])
        .inner(vec![
            named_node(1.0, 1.0, "other_start"),
            named_node(2.0, 1.0, "11"),
            named_node(2.0, 2.0, "12"),
            named_node(1.0, 2.0, "13"),
            named_node(1.0, 1.0, "other_start"),
        ])
        .relation_id
        .into();
    if let osmpbfreader::OsmObj::Relation(ref relation) = builder.objects[&rel_id] {
        let multipolygon = build_boundary(&relation, &builder.objects);
        assert!(multipolygon.is_some());
        let multipolygon = multipolygon.unwrap();
        assert_eq!(multipolygon.0.len(), 1);
        assert_eq!(multipolygon.area(), 15.);
    } else {
        assert!(false); //this should not happen
    }
}

#[test]
fn test_build_two_boundaries_with_one_hole() {
    use geo::algorithm::area::Area;
    let mut builder = osm_builder::OsmBuilder::new();
    let rel_id = builder
        .relation()
        .outer(vec![
            named_node(0.0, 0.0, "start"),
            named_node(4.0, 0.0, "1"),
            named_node(4.0, 4.0, "2"),
            named_node(0.0, 4.0, "3"),
            named_node(0.0, 0.0, "start"),
        ])
        .inner(vec![
            named_node(1.0, 1.0, "other_start"),
            named_node(2.0, 1.0, "11"),
            named_node(2.0, 2.0, "12"),
            named_node(1.0, 2.0, "13"),
            named_node(1.0, 1.0, "other_start"),
        ])
        .outer(vec![
            named_node(4.0, 4.0, "yet_another_start"),
            named_node(8.0, 4.0, "4"),
            named_node(8.0, 8.0, "5"),
            named_node(4.0, 8.0, "6"),
            named_node(0.0, 0.0, "yet_another_start"),
        ])
        .relation_id
        .into();
    if let osmpbfreader::OsmObj::Relation(ref relation) = builder.objects[&rel_id] {
        let multipolygon = build_boundary(&relation, &builder.objects);
        assert!(multipolygon.is_some());
        let multipolygon = multipolygon.unwrap();
        assert_eq!(multipolygon.0.len(), 2);
        assert_eq!(multipolygon.area(), 31.);
    } else {
        assert!(false); //this should not happen
    }
}

#[test]
fn test_build_one_boundary_with_two_holes() {
    use geo::algorithm::area::Area;
    let mut builder = osm_builder::OsmBuilder::new();
    let rel_id = builder
        .relation()
        .outer(vec![
            named_node(0.0, 0.0, "start"),
            named_node(4.0, 0.0, "1"),
            named_node(4.0, 4.0, "2"),
            named_node(0.0, 4.0, "3"),
            named_node(0.0, 0.0, "start"),
        ])
        .inner(vec![
            named_node(1.0, 1.0, "other_start"),
            named_node(2.0, 1.0, "11"),
            named_node(2.0, 2.0, "12"),
            named_node(1.0, 2.0, "13"),
            named_node(1.0, 1.0, "other_start"),
        ])
        .inner(vec![
            named_node(2.0, 2.0, "yet_another_start"),
            named_node(3.0, 2.0, "4"),
            named_node(3.0, 3.0, "5"),
            named_node(2.0, 3.0, "6"),
            named_node(2.0, 2.0, "yet_another_start"),
        ])
        .relation_id
        .into();
    if let osmpbfreader::OsmObj::Relation(ref relation) = builder.objects[&rel_id] {
        let multipolygon = build_boundary(&relation, &builder.objects);
        assert!(multipolygon.is_some());
        let multipolygon = multipolygon.unwrap();
        assert_eq!(multipolygon.0.len(), 1);
        assert_eq!(multipolygon.area(), 14.);
    } else {
        assert!(false); //this should not happen
    }
}

#[test]
fn test_build_two_boundaries_with_two_holes() {
    /// this is a shirt button (one geom with two holes) and anoter geom without hole
    use geo::algorithm::area::Area;
    let mut builder = osm_builder::OsmBuilder::new();
    let rel_id = builder
        .relation()
        .outer(vec![
            named_node(0.0, 0.0, "start"),
            named_node(4.0, 0.0, "1"),
            named_node(4.0, 4.0, "2"),
            named_node(0.0, 4.0, "3"),
            named_node(0.0, 0.0, "start"),
        ])
        .inner(vec![
            named_node(1.0, 1.0, "other_start"),
            named_node(2.0, 1.0, "11"),
            named_node(2.0, 2.0, "12"),
            named_node(1.0, 2.0, "13"),
            named_node(1.0, 1.0, "other_start"),
        ])
        .inner(vec![
            named_node(2.0, 2.0, "another_start"),
            named_node(3.0, 2.0, "4"),
            named_node(3.0, 3.0, "5"),
            named_node(2.0, 3.0, "6"),
            named_node(2.0, 2.0, "another_start"),
        ])
        .outer(vec![
            named_node(4.0, 4.0, "yet_another_start"),
            named_node(8.0, 4.0, "14"),
            named_node(8.0, 8.0, "15"),
            named_node(4.0, 8.0, "16"),
            named_node(0.0, 0.0, "yet_another_start"),
        ])
        .relation_id
        .into();
    if let osmpbfreader::OsmObj::Relation(ref relation) = builder.objects[&rel_id] {
        let multipolygon = build_boundary(&relation, &builder.objects);
        assert!(multipolygon.is_some());
        let multipolygon = multipolygon.unwrap();
        assert_eq!(multipolygon.0.len(), 2);
        assert_eq!(multipolygon.area(), 30.);
    } else {
        assert!(false); //this should not happen
    }
}
