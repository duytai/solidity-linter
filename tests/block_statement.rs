mod setup;

use std::io;
use setup::setup_cfg;
use ssa::core::{ Shape, Vertex };

#[test]
fn complex_block() -> io::Result<()> {
    setup_cfg("block_1.sol", 55, |cfg| {
        let vertices = cfg.get_vertices();
        let edges = cfg.get_edges();
        assert_eq!(vertices.len(), 10);
        assert_eq!(edges.len(), 9);
        let function_calls = vertices.iter()
            .filter(|v| {
                v.get_shape() == &Shape::DoubleCircle
            })
            .collect::<Vec<&Vertex>>();
        assert_eq!(function_calls.len(), 4);
    })?;
    Ok(())
}
