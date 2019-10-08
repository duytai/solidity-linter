use crate::dot::Dot;
use crate::cfg::ControlFlowGraph;
use crate::dfg::DataFlowGraph;
use crate::core::{
    DataLink,
    Dictionary,
    SmartContractQuery,
    Action,
    Variable,
    Vertex,
};

use std::collections::{
    HashMap,
    HashSet,
};

#[derive(Debug)]
enum StackContext {
    Push(u32),
    Pop(u32),
}

pub struct Network<'a> {
    dict: &'a Dictionary<'a>,
    links: HashSet<DataLink>,
    dfgs: HashMap<u32, DataFlowGraph<'a>>,
    dot: Dot,
    contract_id: u32,
    context: HashMap<(u32, u32), StackContext>,
    all_actions: HashMap<u32, Vec<Action>>,
    all_indexes: HashMap<u32, Vec<u32>>,
    all_execution_paths: Vec<Vec<u32>>,
    all_fcalls: HashMap<u32, Vec<u32>>,
    all_returns: HashMap<u32, Vec<u32>>,
    all_vertices: HashMap<u32, Vertex>,
    all_defined_parameters: HashMap<u32, Vec<u32>>,
    all_states: HashSet<Variable>,
}

impl<'a> Network<'a> {
    pub fn new(dict: &'a Dictionary, contract_id: u32) -> Self {
        let mut network = Network {
            dict,
            links: HashSet::new(),
            dfgs: HashMap::new(),
            dot: Dot::new(),
            context: HashMap::new(),
            all_actions: HashMap::new(),
            all_indexes: HashMap::new(),
            all_execution_paths: vec![],
            all_fcalls: HashMap::new(),
            all_returns: HashMap::new(),
            all_vertices: HashMap::new(),
            all_defined_parameters: HashMap::new(),
            all_states: HashSet::new(),
            contract_id,
        };
        network.find_links();
        network
    }

    pub fn get_all_vertices(&self) -> &HashMap<u32, Vertex> {
        &self.all_vertices
    }

    pub fn get_all_defined_parameters(&self) -> &HashMap<u32, Vec<u32>> {
        &self.all_defined_parameters
    }

    pub fn get_all_returns(&self) -> &HashMap<u32, Vec<u32>> {
        &self.all_returns
    }

    pub fn get_all_fcall(&self) -> &HashMap<u32, Vec<u32>> {
        &self.all_fcalls
    }

    pub fn get_all_executions(&self) -> &Vec<Vec<u32>> {
        &self.all_execution_paths
    }

    pub fn get_all_indexes(&self) -> &HashMap<u32, Vec<u32>> {
        &self.all_indexes
    }

    pub fn get_all_actions(&self) -> &HashMap<u32, Vec<Action>> {
        &self.all_actions
    }

    pub fn get_all_states(&self) -> &HashSet<Variable> {
        &self.all_states
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

    fn find_assignment_links(&mut self) -> HashSet<DataLink> {
        let mut assignment_links = HashSet::new();
        for (vertex_id, actions) in self.all_actions.iter() {
            let mut kill_variables = HashSet::new();
            let mut use_variables = HashSet::new();
            for action in actions {
                match action {
                    Action::Use(variable, _) => {
                        use_variables.insert(variable.clone());
                    },
                    Action::Kill(variable, _) => {
                        kill_variables.insert(variable.clone());
                    },
                }
            }
            let from = (kill_variables, *vertex_id);
            let to = (use_variables, *vertex_id);
            assignment_links.extend(Variable::links(from, to));
        }
        assignment_links
    }

    fn find_index_links(&mut self) -> HashSet<DataLink> {
        let mut index_links = HashSet::new();
        for (index_id, params) in self.all_indexes.iter() {
            let index_variables = self.get_variables(index_id);
            for index_param_id in &params[2..] {
                let param_variables = self.get_variables(index_param_id);
                let from = (index_variables.clone(), *index_id);
                let to = (param_variables, *index_param_id);
                index_links.extend(Variable::mix(from, to));
            }
            {
                let param_variables = self.get_variables(&params[1]);
                let from = (index_variables.clone(), *index_id);
                let to = (param_variables, params[1]);
                index_links.extend(Variable::links(from, to));
            }
            self.dict.walker_at(params[0]).map(|walker| {
                if walker.node.name != "IndexAccess" {
                    let from = (index_variables.clone(), params[0]);
                    let to = (index_variables, *index_id);
                    index_links.extend(Variable::links(from, to));
                }
            });
        }
        index_links
    }

    fn find_fcall_links(&mut self) -> HashSet<DataLink>  {
        let mut context = HashMap::new();
        let mut fcall_links = HashSet::new();
        for (fcall_id, invoked_parameters) in self.all_fcalls.iter() {
            let fcall_variables = self.get_variables(fcall_id);
            self.dict.walker_at(*fcall_id).map(|walker| {
                let walkers = walker.direct_childs(|_| true);
                let declaration = walkers[0].node.attributes["referencedDeclaration"].as_u32();
                let is_user_defined = declaration.and_then(|declaration| self.all_returns.get(&declaration)).is_some();
                match is_user_defined {
                    false => {
                        for param_id in (&invoked_parameters[2..]).iter() {
                            let param_variables = self.get_variables(param_id);
                            let from = (fcall_variables.clone(), *fcall_id);
                            let to = (param_variables, *param_id);
                            fcall_links.extend(Variable::mix(from, to));
                        }
                        {
                            let param_variables = self.get_variables(&invoked_parameters[1]);
                            let from = (fcall_variables.clone(), *fcall_id);
                            let to = (param_variables, invoked_parameters[1]);
                            fcall_links.extend(Variable::links(from, to));
                        }
                        self.dict.walker_at(invoked_parameters[0]).map(|walker| {
                            if walker.node.name != "FunctionCall" {
                                let from = (fcall_variables.clone(), invoked_parameters[0]);
                                let to = (fcall_variables, *fcall_id);
                                fcall_links.extend(Variable::links(from, to));
                            }
                        });
                    },
                    true => {
                        let declaration = declaration.unwrap();
                        let returns = self.all_returns.get(&declaration).unwrap();
                        let defined_parameters = self.all_defined_parameters.get(&declaration).unwrap();
                        for return_id in returns {
                            let return_variables = self.get_variables(return_id);
                            let from = (fcall_variables.clone(), *fcall_id);
                            let to = (return_variables, *return_id);
                            let tmp_links = Variable::links(from, to);
                            for link in tmp_links.iter() {
                                let (_, from) = link.get_from();
                                let (_, to) = link.get_to();
                                context.insert((*from, *to), StackContext::Push(*fcall_id));
                            }
                            fcall_links.extend(tmp_links);
                        }
                        let defined_len = defined_parameters.len();
                        let invoked_len = invoked_parameters.len();
                        for idx in 0..invoked_len - 2 {
                            let defined_parameter_variables = self.get_variables(&defined_parameters[defined_len - idx - 1]);
                            let invoked_parameter_variables = self.get_variables(&invoked_parameters[invoked_len - idx - 1]);
                            let from = (defined_parameter_variables, defined_parameters[defined_len - idx - 1]);
                            let to = (invoked_parameter_variables, invoked_parameters[invoked_len - idx - 1]);
                            let tmp_links = Variable::links(from, to);
                            for link in tmp_links.iter() {
                                let (_, from) = link.get_from();
                                let (_, to) = link.get_to();
                                context.insert((*from, *to), StackContext::Pop(*fcall_id));
                            }
                            fcall_links.extend(tmp_links);
                        }
                        self.dict.walker_at(invoked_parameters[0]).map(|walker| {
                            if walker.node.name != "FunctionCall" {
                                let from = (fcall_variables.clone(), invoked_parameters[0]);
                                let to = (fcall_variables, *fcall_id);
                                fcall_links.extend(Variable::links(from, to));
                            }
                        });
                    }
                }
            });
        }
        self.context = context;
        fcall_links
    }

    fn find_external_links(&mut self) -> HashSet<DataLink> {
        let mut external_links = HashSet::new();
        external_links.extend(self.find_assignment_links());
        external_links.extend(self.find_index_links());
        external_links.extend(self.find_fcall_links());
        external_links
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
        for (_, dfg) in self.dfgs.iter() {
            let cfg = dfg.get_cfg();
            self.all_actions.extend(dfg.get_new_actions().clone());
            self.all_execution_paths.extend(cfg.get_execution_paths().clone());
            self.all_indexes.extend(cfg.get_indexes().clone());
            self.all_fcalls.extend(cfg.get_fcalls().clone());
            self.all_returns.extend(cfg.get_returns().clone());
            self.all_defined_parameters.extend(cfg.get_parameters().clone());
            self.all_vertices.extend(cfg.get_vertices().clone());
        }
        let state_ids = self.dict.find_ids(SmartContractQuery::StatesByContractId(self.contract_id));
        for state_id in state_ids.iter() {
            let variables = self.get_variables(state_id);
            self.all_states.extend(variables);
        } 
        let external_links = self.find_external_links();
        self.links.extend(internal_links);
        self.links.extend(external_links);
    }

    fn network_traverse(
        &self,
        source: (Variable, u32),
        all_links: &HashMap<(Variable, u32), Vec<(Variable, u32)>>,
        mut visited: HashSet<((Variable, u32), Vec<u32>)>,
        stack: Vec<u32>,
        mut execution_path: Vec<(Variable, u32)>,
        execution_paths: &mut Vec<Vec<(Variable, u32)>>,
    ) {
        visited.insert((source.clone(), stack.clone()));
        execution_path.push(source.clone());
        if let Some(sinks) = all_links.get(&source) {
            for sink in sinks {
                let mut valid_stack = true;
                let mut context_stack = stack.clone();
                if let Some(stack_item) = self.context.get(&(source.1, sink.1)) {
                    match stack_item {
                        StackContext::Push(id) => {
                            context_stack.push(*id);
                        },
                        StackContext::Pop(id) => {
                            if let Some(stack_top) = context_stack.pop() {
                                valid_stack = &stack_top == id;
                            }
                        },
                    }
                }
                if valid_stack && !visited.contains(&(sink.clone(), context_stack.clone())) {
                    self.network_traverse(
                        sink.clone(),
                        all_links,
                        visited.clone(),
                        context_stack,
                        execution_path.clone(),
                        execution_paths,
                    );
                }
            }
        } else {
            execution_paths.push(execution_path);
        }
    }

    pub fn traverse(&self, source: (Variable, u32)) -> Vec<Vec<(Variable, u32)>> {
        let mut all_links: HashMap<(Variable, u32), Vec<(Variable, u32)>> = HashMap::new();
        for link in self.links.iter() {
            if let Some(v) = all_links.get_mut(link.get_from()) {
                v.push(link.get_to().clone());
            } else {
                let from = link.get_from().clone();
                let to = link.get_to().clone();
                all_links.insert(from, vec![to]);
            }
        }
        let mut execution_paths = vec![];
        self.network_traverse(
            source,
            &all_links,
            HashSet::new(),
            vec![],
            vec![],
            &mut execution_paths,
        );
        execution_paths
    }

    pub fn get_variables(&self, id: &u32) -> HashSet<Variable> {
        let mut variables = HashSet::new();
        if let Some(actions) = self.all_actions.get(id) {
            for action in actions.iter() {
                match action {
                    Action::Use(variable, _) => {
                        variables.insert(variable.clone());
                    },
                    Action::Kill(variable, _) => {
                        variables.insert(variable.clone());
                    },
                }
            }
        }
        variables
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
