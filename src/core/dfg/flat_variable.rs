extern crate regex;
use regex::Regex;
use std::collections::HashMap;
use std::collections::HashSet;
use crate::core::{
    Dictionary,
    SmartContractQuery,
    Member,
    Walker,
    Utils,
    Variable,
};

pub struct FlatVariable<'a> {
    dict: &'a Dictionary<'a>,
    flats: Vec<(Vec<Member>, String, String)>,
    attributes: Vec<String>,
}

impl<'a> FlatVariable<'a> {
    pub fn new(walker: &Walker, dict: &'a Dictionary) -> Self {
        let mut flat_variable = FlatVariable { dict, flats: vec![], attributes: vec![] };
        let root_walker = Utils::find_root_walker(walker, dict);
        let declaration = root_walker.node.attributes["referencedDeclaration"].as_u32();
        let mut members = vec![];
        let mut attributes = vec![];
        let attribute = root_walker.node.attributes["value"].as_str()
            .or(root_walker.node.attributes["name"].as_str()).unwrap();
        attributes.push(attribute.to_string());
        if root_walker.node.name == "VariableDeclaration" {
            members.push(Member::Reference(root_walker.node.id));
        } else {
            if declaration.and_then(|declaration| dict.walker_at(declaration)).is_none() {
                members.push(Member::Global(attribute.to_string()));
            } else {
                members.push(Member::Reference(declaration.unwrap()));
            }
        }
        flat_variable.update_flats(&Utils::normalize_kind(&root_walker), members, attributes);
        flat_variable.update_attributes(walker, dict);
        flat_variable
    }

    pub fn get_variables(&self) -> HashSet<Variable> {
        let mut ret = HashSet::new();
        println!("Found: {:?}", self.attributes);
        for flat in self.flats.iter() {
            println!("\t{:?}", flat);
        }
        for (members, attributes, kind) in self.flats.iter() {
            let joined_1 = self.attributes.join(".");
            let joined_2 = format!("{}.", joined_1);
            if attributes == &joined_1 || attributes.starts_with(&joined_2) {
                let variable = Variable::new(
                    members.clone(),
                    attributes.clone(),
                    kind.clone(),
                    joined_1,
                );
                ret.insert(variable);
            }
        }
        if ret.is_empty() {
            println!("ERROR on decode variables");
        }
        println!("==> Flatted Varariable <==");
        for r in ret.iter() {
            println!("\t{:?}", r);
        }
        ret
    } 

    fn update_attributes(&mut self, walker: &Walker, dict: &Dictionary) {
        match walker.node.name {
            "IndexAccess" => {
                self.attributes.insert(0, "$".to_string());
            },
            "MemberAccess" => {
                let member_name = walker.node.attributes["member_name"].as_str().unwrap();
                self.attributes.insert(0, member_name.to_string());
            },
            "Identifier" => {
                let value = walker.node.attributes["value"].as_str().unwrap();
                self.attributes.insert(0, value.to_string());
            },
            "VariableDeclaration" => {
                let name = walker.node.attributes["name"].as_str().unwrap();
                self.attributes.insert(0, name.to_string());
            },
            "FunctionCall" => {
                let type_conversion = walker.node.attributes["type_conversion"].as_bool().unwrap_or(false);
                if type_conversion {
                    let to_kind = walker.node.attributes["type"].as_str().unwrap();
                    self.attributes.insert(0, to_kind.to_string());
                }
            },
            _ => {}
        }
        walker.direct_childs(|_| true).get(0).map(|walker| {
            self.update_attributes(walker, dict);
        });
    }

    fn update_flats(&mut self, kind: &str, mut members: Vec<Member>, mut attributes: Vec<String>) {
        let struct_regex = Regex::new(r"^struct ([^\[\]]*)((\[\])*)").unwrap();
        let mapping_regex = Regex::new(r"^mapping\(.+\)((\[\])*)").unwrap();
        let contract_regex = Regex::new(r"^contract ([^\[\]]*)((\[\])*)").unwrap();
        let array_regex = Regex::new(r"([^\[]+)((\[\])+)$").unwrap();
        let conversion_regex = Regex::new(r"^type\((.+)\)$").unwrap();
        let matches = (
            struct_regex.is_match(kind),
            mapping_regex.is_match(kind),
            contract_regex.is_match(kind),
            array_regex.is_match(kind),
            conversion_regex.is_match(kind),
        );
        match matches {
            (true, _, _, _, _) => {
                for cap in struct_regex.captures_iter(kind) {
                    let struct_kind = (&cap[1]).to_string();
                    let dimension = (&cap[2]).len() / 2;
                    for _ in 0..dimension {
                        members.push(Member::IndexAccess);
                        attributes.push(String::from("$"));
                    }
                    self.dict.find_walkers(SmartContractQuery::StructByName(struct_kind)).get(0).map(|walker| {
                        for walker in walker.direct_childs(|_| true) {
                            let mut members = members.clone();
                            let mut attributes = attributes.clone();
                            let name = walker.node.attributes["name"].as_str().unwrap_or("");
                            members.push(Member::Reference(walker.node.id));
                            attributes.push(name.to_string());
                            self.update_flats(&Utils::normalize_kind(&walker), members, attributes);
                        }
                    });
                }
            },
            (_, true, _, _, _) => {
                let mut state = (0, 0, 0); // (depth, from, to)
                for i in 0..kind.len() {
                    if kind[0..=i].ends_with("(") {
                        state.0 += 1;
                    }
                    if kind[0..=i].ends_with(")") {
                        state.0 -= 1;
                    }
                    if state.0 == 1 && kind[0..=i].ends_with("=>") {
                        state.1 = i + 1;
                    }
                    if state.0 == 0 && kind[0..=i].ends_with(")") {
                       state.2 = i - 1;
                    }
                }
                let mapping_kind = String::from(&kind[state.1..=state.2]);
                for cap in mapping_regex.captures_iter(kind) {
                    let dimension = (&cap[1]).len() / 2 + 1;
                    for _ in 0..dimension {
                        members.push(Member::IndexAccess);
                        attributes.push(String::from("$"));
                    }
                    self.update_flats(&mapping_kind.trim(), members.clone(), attributes.clone());
                }
            },
            (_, _, true, _, _) => {
                for cap in contract_regex.captures_iter(kind) {
                    let contract_kind = (&cap[1]).to_string();
                    let dimension = (&cap[2]).len() / 2;
                    for _ in 0..dimension {
                        members.push(Member::IndexAccess);
                        attributes.push(String::from("$"));
                    }
                    self.dict.find_ids(SmartContractQuery::ContractByName(contract_kind)).get(0).map(|contract_id| {
                        {
                            let mut members = members.clone();
                            let mut attributes = attributes.clone();
                            members.push(Member::Global("balance".to_string()));
                            attributes.push("balance".to_string());
                            self.update_flats("uint", members, attributes);
                        }
                        for walker in self.dict.find_walkers(SmartContractQuery::StatesByContractId(*contract_id)) {
                            let mut members = members.clone();
                            let mut attributes = attributes.clone();
                            let name = walker.node.attributes["name"].as_str().unwrap_or("");
                            members.push(Member::Reference(walker.node.id));
                            attributes.push(name.to_string());
                            self.update_flats(&Utils::normalize_kind(&walker), members, attributes);
                        }
                        for walker in self.dict.find_walkers(SmartContractQuery::FunctionsByContractId(*contract_id)) {
                            let mut members = members.clone();
                            let mut attributes = attributes.clone();
                            let name = walker.node.attributes["name"].as_str().unwrap_or("");
                            members.push(Member::Reference(walker.node.id));
                            attributes.push(name.to_string());
                            walker.direct_childs(|_| true).get(1).map(|walker| {
                                let walkers = walker.direct_childs(|_| true);
                                if walkers.is_empty() {
                                    self.update_flats("void", members, attributes);
                                } else {
                                    let walker = &walkers[0];
                                    self.update_flats(&Utils::normalize_kind(&walker), members, attributes);
                                }
                            });
                        }
                    });
                } 
            },
            (_, _, _, true, _) => {
                for cap in array_regex.captures_iter(kind) {
                    let dimension = (&cap[2]).len() / 2;
                    let mut array_kind = (&cap[1]).to_string();
                    for _ in 0..dimension - 1 {
                        array_kind.push_str("[]");
                    }
                    let properties = vec![
                        (Member::IndexAccess, String::from("$"), array_kind),
                        (Member::Global(String::from("push")), String::from("push"), String::from("void")),
                    ];
                    for prop in properties {
                        let mut members = members.clone();
                        let mut attributes = attributes.clone();
                        members.push(prop.0);
                        attributes.push(prop.1);
                        self.update_flats(&prop.2, members, attributes);
                    }
                }
            },
            (_, _, _, _, true) => {
                for cap in conversion_regex.captures_iter(kind) {
                    let inner_kind = (&cap[1]).to_string();
                    self.update_flats(&inner_kind, members.clone(), attributes.clone());
                }
            },
            _ => {
                let mut properties = HashMap::new();
                properties.insert("block", vec![
                  ("blockhash", "bytes32"),
                  ("coinbase", "address"),
                  ("difficulty", "uint256"),
                  ("gaslimit", "uint256"),
                  ("number", "uint256"),
                  ("timestamp", "uint256"),
                ]);
                properties.insert("msg", vec![
                  ("data", "bytes"),
                  ("gas", "uint256"),
                  ("sender", "address"),
                  ("sig", "bytes4"),
                  ("value", "uint256"),
                ]);
                properties.insert("tx", vec![
                  ("gasprice", "uint256"),
                  ("origin", "address"),
                ]);
                properties.insert("abi", vec![
                  ("encode", "bytes"),
                  ("encodePacked", "bytes"),
                  ("encodeWithSelector", "bytes"),
                  ("encodeWithSignature", "bytes"),
                  ("encodeWithSelector", "bytes"),
                ]);
                properties.insert("address", vec![
                  ("balance", "uint256"),
                  ("transfer", "void"),
                  ("send", "bool"),
                  ("call", "bool"),
                  ("callcode", "bool"),
                  ("delegatecall", "bool"),
                ]);
                if let Some(property) = properties.get(kind) {
                    for prop in property.iter() {
                        let mut members = members.clone();
                        let mut attributes = attributes.clone();
                        members.push(Member::Global(prop.0.to_string()));
                        attributes.push(prop.0.to_string());
                        self.update_flats(prop.1, members, attributes);
                    }
                } else {
                    let query = SmartContractQuery::LibraryByKind(kind.to_string());
                    if let Some(walker) = self.dict.find_walkers(query).get(0) {
                        for walker in walker.direct_childs(|_| true) {
                            let mut members = members.clone();
                            let mut attributes = attributes.clone();
                            let name = walker.node.attributes["name"].as_str().unwrap_or("");
                            members.push(Member::Reference(walker.node.id));
                            attributes.push(name.to_string());
                            walker.direct_childs(|_| true).get(1).map(|walker| {
                                let walkers = walker.direct_childs(|_| true);
                                let kind = match walkers.is_empty() {
                                    true => String::from("void"),
                                    false => Utils::normalize_kind(&walkers[0]).to_string(),
                                };
                                let flat = (members, attributes.join("."), kind);
                                self.flats.push(flat);
                            });
                        }
                    } else {
                        let flat = (members, attributes.join("."), kind.to_string());
                        self.flats.push(flat);
                    }
                }
            }
        }
    }
}
