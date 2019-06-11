#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub enum Shape {
    Point,
    Box,
    Diamond,
    DoubleCircle,
    Mdiamond,
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct Vertex {
    pub id: u32,
    pub source: String,
    pub shape: Shape,
}

impl Vertex {
    pub fn new(id: u32, source: &str, shape: Shape) -> Self {
        Vertex {
            id,
            shape,
            source: source.to_string(),
        }
    }

    pub fn to_string(&self) -> String {
        let shape = match self.shape {
            Shape::Point => "point",
            Shape::Box => "box",
            Shape::Diamond => "diamond",
            Shape::DoubleCircle => "doublecircle",
            Shape::Mdiamond => "Mdiamond",
        };
        format!("  {}[label={:?}, shape=\"{}\"];\n", self.id, self.source, shape)
    }
}
