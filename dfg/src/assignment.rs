use std::collections::HashSet;
use crate::variable::{ Variable };
use crate::core::{ Walker, Dictionary };

/// Operator in an assignment statement
///
/// - `Operator::Equal` : the variable in LHS clears it own value and create a data dependency on all variables in RHS
/// ```
/// x = y;
/// KILL(x), USE(Y)
/// ```
/// - `Operator::Other` : the variable in LHS is modified by using both its value and
/// RHS
/// ```
/// x += y;
/// USE(x), USE(y)
/// ```
#[derive(Debug, PartialEq, Eq)]
pub enum Operator {
    /// Operator =
    Equal,
    /// Other operators: |=, ^=, &=, <<=, >>=, +=, -=, *=, /=, %=
    Other,
}

/// The statement edits the flow of data in a solidity program 
///
/// The value of a variable can be passed to other variables through assignment statement or
/// combined with other variables. The assignment not only standard expression (LHS = RHS) but also LHS without RHS. The
/// full list of solidity tokens possibly including the assignment statement:
///
/// 1. __Assignment__ : a standard assignment inside body of a function.
///
/// ```
/// x = y + 20;
/// ```
///
/// 2. __VariableDeclaration__: a state variable declaration in a contract
///
/// ```
/// contract Sample {
///   uint totalSupply = 0;
/// }
/// ```
///
/// 3. __ParameterList__: a list of parameters of a function
///
/// ```
/// contract Sample {
///   function add(uint x, uint y) returns(uint) {}
/// }
/// ```
/// 4. __VariableDeclarationStatement__: local variable declaration
///
/// ```
/// uint x = y + 10;
/// ```

#[derive(Debug)]
pub struct Assignment {
    /// a list of variables in LHS of a assignment
    lhs: HashSet<Variable>,
    /// a list of variables in RHS of a assignment
    rhs: HashSet<Variable>,
    /// the operator in a assignment
    op: Operator,
}

impl Assignment {
    /// Export current assignment to a tuple 
    pub fn to_tuple(&self) -> (&HashSet<Variable>, &HashSet<Variable>, &Operator) {
        (&self.lhs, &self.rhs, &self.op)
    }

    /// Find all variables in current walker, the dictionary is used to identify global variables 
    pub fn parse(walker: &Walker, dict: &Dictionary) -> Vec<Assignment> {
        let mut assignments = vec![];
        match walker.node.name {
            // state variables
            "VariableDeclaration" => {
                let op = Operator::Equal;
                let lhs = Variable::parse(&walker, dict);
                let rhs = HashSet::new();
                assignments.push(Assignment { lhs, rhs, op });
            },
            // variables from parameters
            "ParameterList" => {
                walker.for_each(|walker, _| {
                    let op = Operator::Equal;
                    let lhs = Variable::parse(&walker, dict);
                    let rhs = HashSet::new();
                    assignments.push(Assignment { lhs, rhs, op });
                });
            },
            // local variable definitions
            "VariableDeclarationStatement" => {
                assignments.push(Assignment::parse_one(&walker, dict));
            },
            // variable assignments
            _ => {
                walker.for_all(|walker| {
                    walker.node.name == "Assignment"
                }, |walkers| {
                    for walker in walkers {
                        assignments.push(Assignment::parse_one(&walker, dict));
                    }
                });
            },
        }
        assignments
    }

    /// Find a assignment of current walker
    fn parse_one(walker: &Walker, dict: &Dictionary) -> Assignment {
        let operator = walker.node.attributes["operator"].as_str().unwrap_or("=");
        let op = match operator {
            "=" => Operator::Equal,
            _ => Operator::Other, 
        };
        let mut lhs = HashSet::new();
        let mut rhs = HashSet::new();
        walker.for_all(|_| { true }, |walkers| {
            lhs.extend(Variable::parse(&walkers[0], dict));
            if walkers.len() >= 2 {
                rhs.extend(Variable::parse(&walkers[1], dict));
            }
        });
        Assignment { lhs, rhs, op }
    }
}
