use crate::dot::Dot;
use crate::cfg::ControlFlowGraph;
use crate::dfg::DataFlowGraph;
use crate::core::{
    DataLink,
    Dictionary,
    SmartContractQuery,
    Action,
};
use std::collections::{
    HashMap,
    HashSet,
};

pub struct Network<'a> {
    dict: &'a Dictionary<'a>,
    links: HashSet<DataLink>,
    dfgs: HashMap<u32, DataFlowGraph<'a>>,
    dot: Dot,
    contract_id: u32,
}

impl<'a> Network<'a> {
    pub fn new(dict: &'a Dictionary, contract_id: u32) -> Self {
        let mut network = Network {
            dict,
            links: HashSet::new(),
            dfgs: HashMap::new(),
            dot: Dot::new(),
            contract_id,
        };
        network.find_links();
        network
    }

    pub fn get_links(&self) -> &HashSet<DataLink> {
        &self.links
    }

    pub fn get_dfgs(&self) -> &HashMap<u32, DataFlowGraph> {
        &self.dfgs
    }

    pub fn get_dict(&self) -> &Dictionary {
        &self.dict
    }

    pub fn get_contract_id(&self) -> u32 {
        self.contract_id
    }

    fn find_external_links(&mut self) -> HashSet<DataLink> {
        let mut all_variables = HashMap::new();
        for (_, dfg) in self.dfgs.iter() {
            all_variables.extend(dfg.get_all_variables());
        }
        for index_id in self.dict.find_ids(SmartContractQuery::IndexesByContractId(self.contract_id)) {
            all_variables.get(&index_id).map(|index_variables| {
                for param_id in self.dict.find_ids(SmartContractQuery::IndexParamsByIndexAccess(index_id)) {
                    all_variables.get(&param_id).map(|param_variables| {
                    });
                }
            });
        }
        HashSet::new()
    } 

    fn find_internal_links(&mut self) -> HashSet<DataLink> {
        let mut links = HashSet::new();
        let function_ids = self.dict.find_ids(SmartContractQuery::FunctionsByContractId(self.contract_id));
        for function_id in function_ids {
            let cfg = ControlFlowGraph::new(self.dict, self.contract_id, function_id);
            let mut dfg = DataFlowGraph::new(cfg);
            links.extend(dfg.find_links());
            self.dfgs.insert(function_id, dfg);
        }
        links
    }

    fn find_links(&mut self) {
        let internal_links = self.find_internal_links();
        let external_links = self.find_external_links();
        self.links.extend(internal_links);
        self.links.extend(external_links);
    }

    pub fn format(&mut self) -> String {
        self.dot.clear();
        for (_, dfg) in self.dfgs.iter() {
            self.dot.add_cfg(dfg.get_cfg());
        }
        self.dot.add_links(&self.links);
        self.dot.format()
    }
}
