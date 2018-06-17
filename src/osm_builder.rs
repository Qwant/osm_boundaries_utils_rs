extern crate osmpbfreader;
use geo::Point;
use std::collections::BTreeMap;

pub fn named_node(lon: f64, lat: f64, name: &'static str) -> (Point<f64>, Option<String>) {
    (Point::new(lon, lat), Some(name.to_string()))
}

pub struct Relation<'a> {
    builder: &'a mut OsmBuilder,
    pub relation_id: osmpbfreader::RelationId,
}

impl<'a> Relation<'a> {
    pub fn outer(&mut self, coords: Vec<(Point<f64>, Option<String>)>) -> &'a mut Relation {
        let id = self.builder.way(coords);
        if let &mut osmpbfreader::OsmObj::Relation(ref mut rel) = self
            .builder
            .objects
            .get_mut(&self.relation_id.into())
            .unwrap()
        {
            rel.refs.push(osmpbfreader::Ref {
                role: "outer".to_string(),
                member: id.into(),
            });
        }
        self
    }
}

impl<'a> Relation<'a> {
    pub fn inner(&mut self, coords: Vec<(Point<f64>, Option<String>)>) -> &'a mut Relation {
        let id = self.builder.way(coords);
        if let &mut osmpbfreader::OsmObj::Relation(ref mut rel) = self
            .builder
            .objects
            .get_mut(&self.relation_id.into())
            .unwrap()
        {
            rel.refs.push(osmpbfreader::Ref {
                role: "inner".to_string(),
                member: id.into(),
            });
        }
        self
    }
}

pub struct OsmBuilder {
    node_id: i64,
    way_id: i64,
    relation_id: i64,
    pub objects: BTreeMap<osmpbfreader::OsmId, osmpbfreader::OsmObj>,
    named_nodes: BTreeMap<String, osmpbfreader::NodeId>,
}

impl OsmBuilder {
    pub fn new() -> OsmBuilder {
        OsmBuilder {
            node_id: 0,
            way_id: 0,
            relation_id: 0,
            objects: BTreeMap::new(),
            named_nodes: BTreeMap::new(),
        }
    }

    pub fn relation(&mut self) -> Relation {
        let id = osmpbfreader::RelationId(self.relation_id);
        let r = osmpbfreader::Relation {
            id: id,
            refs: vec![],
            tags: osmpbfreader::Tags::new(),
        };
        self.relation_id += 1;
        self.objects.insert(id.into(), r.into());
        Relation {
            builder: self,
            relation_id: id,
        }
    }

    pub fn way(&mut self, coords: Vec<(Point<f64>, Option<String>)>) -> osmpbfreader::WayId {
        let nodes = coords
            .into_iter()
            .map(|pair| self.node(pair.0, pair.1))
            .collect::<Vec<_>>();
        let id = osmpbfreader::WayId(self.way_id);
        let w = osmpbfreader::Way {
            id: id,
            nodes: nodes,
            tags: osmpbfreader::Tags::new(),
        };
        self.way_id += 1;
        self.objects.insert(id.into(), w.into());
        id
    }

    pub fn node(&mut self, coord: Point<f64>, name: Option<String>) -> osmpbfreader::NodeId {
        if let Some(value) = name.as_ref().and_then(|n| self.named_nodes.get(n)) {
            return *value;
        }
        let id = osmpbfreader::NodeId(self.node_id);
        let n = osmpbfreader::Node {
            id: id,
            decimicro_lat: (coord.lat() * 1e7) as i32,
            decimicro_lon: (coord.lng() * 1e7) as i32,
            tags: osmpbfreader::Tags::new(),
        };
        self.node_id += 1;
        self.objects.insert(id.into(), n.into());
        if let Some(ref n) = name {
            self.named_nodes.insert(n.clone(), id);
        }
        id
    }
}
