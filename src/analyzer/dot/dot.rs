use crate::{
    analyzer::{ Analyzer, State },
};

pub struct Dot {}

impl Dot {
    pub fn new() -> Self {
        Dot {}
    }
}

impl Analyzer for Dot {
    fn analyze(&mut self, state: &mut State) {
        let mut vertices_str = String::from("");
        let mut edges_str = String::from("");
        let State { edges, vertices, links, .. } =  state;
        for edge in edges.iter() {
            let edge_str = format!("  {} -> {};\n", edge.0, edge.1);
            edges_str.push_str(&edge_str);
        }
        for vertice in vertices.iter() {
            vertices_str.push_str(&vertice.to_string());
        }
        if let Some(links) = links {
            for link in links.iter() {
                let label = &link.var.source;
                let edge_str = format!("  {} -> {}[label=\"{}\", style=dotted];\n", link.from, link.to, label);
                edges_str.push_str(&edge_str);
            }
        }

        println!("digraph {{\n{0}{1}}}", vertices_str, edges_str);
    }
}

