use std::collections::{ HashSet, HashMap };
use crate::{
    vertex::{ Vertex, Shape },
    dict::Dictionary,
    oracle::{ Oracle },
};
use super::{
    variable::{ Variable, VariableComparison },
    assignment::{ Assignment, Operator },
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Action {
    Use(Variable, u32),
    Kill(Variable, u32),
}

pub struct DataFlowGraph {}

impl DataFlowGraph {
    pub fn new() -> Self {
        DataFlowGraph {}
    }

    pub fn find_assignments(&self, id: u32, dict: &Dictionary) -> Vec<Assignment> {
        let walker = dict.lookup(id).unwrap();
        Assignment::parse(walker, dict)
    }

    pub fn find_variables(&self, id: u32, dict: &Dictionary) -> HashSet<Variable> {
        let walker = dict.lookup(id).unwrap();
        Variable::parse(walker, dict)
    }

    pub fn find_parameters(&self, id: u32, dict: &Dictionary) -> HashSet<Variable> {
        let walker = dict.lookup(id).unwrap();
        let mut variables = HashSet::new();
        walker.for_all(|_| { true }, |walkers| {
            for walker in &walkers[1..] {
                let vars = Variable::parse(walker, dict);
                variables.extend(vars);
            }
        });
        variables
    }
}

impl Oracle for DataFlowGraph {
    fn analyze(
        &mut self,
        edges: &HashSet<(u32, u32)>,
        vertices: &HashSet<Vertex>,
        dict: &Dictionary
    ) {
        let stop = 1000000;
        let mut visited: HashSet<u32> = HashSet::new();
        let mut stack: Vec<(u32, u32, HashSet<Action>)> = vec![];
        let mut parents: HashMap<u32, Vec<u32>> = HashMap::new();
        let mut tables: HashMap<u32, HashSet<Action>> = HashMap::new();
        let actions: HashSet<Action> = HashSet::new(); 
        for vertex in vertices {
            tables.insert(vertex.id, HashSet::new());
        }
        for (from, to) in edges {
            match parents.get_mut(to) {
                Some(v) => { v.push(*from); },
                None => { parents.insert(*to, vec![*from]); },
            }
        }
        if let Some(parents) = parents.get(&stop) {
            for parent in parents {
                stack.push((stop, *parent, actions.clone()));
            }
        } 
        while stack.len() > 0 {
            let (from, id, mut actions) = stack.pop().unwrap();
            let vertex = vertices.iter().find(|v| v.id == id).unwrap();
            let pre_table = tables.get(&from).unwrap().clone();
            let cur_table = tables.get_mut(&id).unwrap();
            let cur_table_len = cur_table.len();
            let mut new_actions: HashSet<Action> = HashSet::new();
            let mut kill_action = None;
            match vertex.shape {
                Shape::DoubleCircle => {
                    for var in self.find_parameters(id, dict) {
                        new_actions.insert(Action::Use(var, id));
                    }
                },
                Shape::Box => {
                    let assignments = self.find_assignments(id, dict);
                    if assignments.len() > 0 {
                        for assignment in assignments {
                            let Assignment { lhs, rhs, op } = assignment;
                            for l in lhs {
                                match op {
                                    Operator::Equal => {
                                        kill_action = Some(Action::Kill(l, id));
                                    },
                                    Operator::Other => {
                                        kill_action = Some(Action::Kill(l.clone(), id));
                                        new_actions.insert(Action::Use(l, id));
                                    }
                                }
                            }
                            for r in rhs {
                                new_actions.insert(Action::Use(r, id));
                            }
                        }
                    } else {
                        for var in self.find_variables(id, dict) {
                            new_actions.insert(Action::Use(var, id));
                        }
                    }
                },
                Shape::Diamond => {},
                Shape::Point => {},
            }
            actions.extend(new_actions.clone());
            cur_table.extend(pre_table);
            cur_table.extend(new_actions);
            if let Some(kill_action) = kill_action {
                cur_table.insert(kill_action.clone());
                if let Action::Kill(kill_var, kill_id) = kill_action {
                    actions.retain(|action| {
                        match action {
                            Action::Use(variable, id) => {
                                match kill_var.contains(variable) {
                                    VariableComparison::Equal => {
                                        println!("LINK {} - {}", id, kill_id);
                                        false
                                    },
                                    VariableComparison::NotEqual => {
                                        true
                                    },
                                    VariableComparison::Partial => {
                                        true
                                    }
                                }
                            },
                            _ => true,
                        }
                    });
                }
            }
            if cur_table.len() != cur_table_len || !visited.contains(&id) {
                visited.insert(id);
                if let Some(parents) = parents.get(&id) {
                    for parent in parents {
                        stack.push((id, *parent, actions.clone()));
                    }
                }
            }
        }
    }
}
