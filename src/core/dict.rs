use crate::core::walker::Walker;
use std::collections::HashMap;

/// Keep inheritance tree and function entry
#[derive(Debug)]
pub struct ContractProp {
    states: Vec<u32>,
    functions: Vec<u32>,
    parents: Vec<u32>,
}

/// Allow searching by node id 
#[derive(Debug)]
pub struct Dictionary<'a> {
    entries: HashMap<u32, Walker<'a>>,
    contracts: HashMap<u32, ContractProp>,
}

impl<'a> Dictionary<'a> {
    /// Create new dictionary
    pub fn new(value: &'a json::JsonValue, sources: &'a HashMap<String, String>) -> Self {
        let mut dict = Dictionary {
            entries: HashMap::new(),
            contracts: HashMap::new(),
        };
        for (name, source) in sources {
            let ast_one = &value["sources"][name]["AST"];
            let walker = Walker::new(ast_one, source);
            dict.traverse(&walker);
        }
        dict
    }

    /// Traverse AST and save data for later searches
    pub fn traverse(&mut self, walker: &Walker<'a>) {
        if walker.node.name == "ContractDefinition" {
            let mut prop = ContractProp {
                states: Vec::new(),
                functions: Vec::new(),
                parents: Vec::new(),
            };
            for walker in walker.direct_childs(|_| true).into_iter() {
                match walker.node.name {
                    "InheritanceSpecifier" | "UsingForDirective" => {
                        let walkers = walker.direct_childs(|_| true);
                        let reference = walkers[0].node.attributes["referencedDeclaration"].as_u32();
                        if let Some(reference) = reference {
                            prop.parents.push(reference);
                        }
                    },
                    "FunctionDefinition" | "ModifierDefinition" => {
                        prop.functions.push(walker.node.id);
                    },
                    "VariableDeclaration" => {
                        prop.states.push(walker.node.id);
                    },
                    _ => {},
                }
            }
            self.contracts.insert(walker.node.id, prop);
        }
        for walker in walker.direct_childs(|_| true).into_iter() {
            self.traverse(&walker);
            self.entries.insert(walker.node.id, walker);
        }
    }

    /// Find walker by node id
    pub fn lookup(&self, id: u32) -> Option<&Walker> {
        self.entries.get(&id)
    }

    /// Find return statements
    pub fn lookup_returns(&self, id: u32) -> Vec<&Walker> {
        let fi = |walker: &Walker, _: &Vec<Walker>| {
            walker.node.name == "Return" || walker.node.name == "PlaceholderStatement"
        };
        let ig = |_: &Walker, _: &Vec<Walker>| false;
        self.entries
            .get(&id)
            .and_then(|walker| match walker.node.name {
                "FunctionDefinition" | "ModifierDefinition" => {
                    let walkers = walker.walk(true, ig, fi)
                        .iter()
                        .filter_map(|w| self.lookup(w.node.id))
                        .collect::<Vec<&Walker>>();
                    Some(walkers)
                },
                _ => None,
            })
            .unwrap_or(vec![])
    } 

    /// Find all function calls start from @id
    pub fn lookup_function_calls(&self, id: u32) -> Vec<&Walker> {
        let fi = |walker: &Walker, _: &Vec<Walker>| {
            walker.node.name == "FunctionCall" || walker.node.name == "ModifierInvocation"
        };
        let ig = |_: &Walker, _: &Vec<Walker>| false;
        self.entries
            .get(&id)
            .and_then(|walker| {
                let walkers = walker.walk(false, ig, fi)
                    .iter()
                    .filter_map(|w| self.lookup(w.node.id))
                    .collect::<Vec<&Walker>>();
                Some(walkers)
            })
            .unwrap_or(vec![])
    }

    /// Find all parameters of a function definition or function call
    pub fn lookup_parameters(&self, id: u32) -> Vec<&Walker> {
        self.entries
            .get(&id)
            .and_then(|walker|  match walker.node.name {
                "FunctionDefinition" | "ModifierDefinition" => {
                    let mut ret = vec![];
                    for (index, walker) in walker.direct_childs(|_| true).iter().enumerate() {
                        if index == 0 && walker.node.name == "ParameterList" {
                            for walker in walker.direct_childs(|_| true).iter() {
                                ret.push(walker.node.id);
                            }
                        }
                    }
                    Some(ret)
                },
                "FunctionCall" | "ModifierInvocation" => {
                    let mut ret = vec![];
                    for (index, walker) in walker.direct_childs(|_| true).into_iter().enumerate() {
                        if index > 0 {
                            ret.push(walker.node.id)
                        }
                    }
                    Some(ret)
                },
                _ => Some(vec![])
            })
            .and_then(|ids| {
                let ret = ids.iter()
                    .map(|id| { self.lookup(*id)})
                    .filter_map(|w| w)
                    .collect::<Vec<&Walker>>();
                Some(ret)
            })
            .unwrap_or(vec![])
    }

    /// Find contract based on function call return type
    pub fn lookup_contract_by_name(&self, name: &str) -> u32 {
        let mut contract_id = None;
        for (id, _) in self.contracts.iter() {
            self.lookup(*id)
                .and_then(|walker| walker.node.attributes["name"].as_str())
                .map(|contract_name| {
                    if name == contract_name {
                        contract_id = Some(*id);
                    }
                });
        }
        contract_id.expect("Contract must exists")
    }

    /// Find scoped functions from id of contract
    /// Start from ContractDefinition node
    pub fn lookup_functions_by_contract_id(&self, id: u32) -> Vec<&Walker> {
        let mut ret = vec![];
        if let Some(prop) = self.contracts.get(&id) {
            for index in (0..prop.functions.len()).rev() {
                ret.push(prop.functions[index]);
            }
            let mut parents = prop.parents.clone();
            loop {
                match parents.pop() {
                    Some(contract_id) => {
                        if let Some(prop) = self.contracts.get(&contract_id) {
                            for index in (0..prop.functions.len()).rev() {
                                ret.push(prop.functions[index]);
                            }
                            parents.extend_from_slice(&prop.parents[..]);
                        }
                    },
                    None => { break; }
                }
            }
        }
        ret.reverse();
        ret.iter()
           .map(|id| { self.lookup(*id)})
           .filter_map(|w| w)
           .collect::<Vec<&Walker>>()
    }

    /// Find a list of states from function_id
    /// Include inherited states
    pub fn lookup_states_by_function_id(&self, id: u32) -> Vec<&Walker> {
        let mut ret = vec![];
        for (_, prop) in self.contracts.iter() {
            if prop.functions.contains(&id) {
                for index in (0..prop.states.len()).rev() {
                    ret.push(prop.states[index]);
                }
                let mut parents = prop.parents.clone();
                loop {
                    match parents.pop() {
                        Some(contract_id) => {
                            if let Some(prop) = self.contracts.get(&contract_id) {
                                for index in (0..prop.states.len()).rev() {
                                    ret.push(prop.states[index]);
                                }
                                parents.extend_from_slice(&prop.parents[..]);
                            }
                        },
                        None => { break; }
                    }
                }
                break;
            }
        }
        ret.reverse();
        ret.iter()
           .map(|id| { self.lookup(*id)})
           .filter_map(|w| w)
           .collect::<Vec<&Walker>>()
    }
}
