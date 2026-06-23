use crate::ast::{ActionStmt, Expr};
use crate::runtime::interner::Symbol;
use std::collections::HashSet;

impl Expr {
    /// Returns the free variables in `self` with respect to `var_binded`
    ///
    /// This is used for:
    /// - Extracting dependencies of each `def` declaration
    /// - Evaluating an expression (substitution based evaluation)
    ///
    /// Args:
    ///     `reactive_names` (`&HashSet<Symbol>`): The set of reactive names
    ///     `var_binded` (`&HashSet<Symbol>`): The set of bound symbols
    ///
    /// Returns:
    ///     `HashSet<Symbol>`: The set of free variables
    pub fn free_var(
        &self,
        reactive_names: &HashSet<Symbol>,
        var_binded: &HashSet<Symbol>,
    ) -> HashSet<Symbol> {
        match self {
            Expr::Literal { .. } | Expr::Table { .. } => HashSet::new(),
            Expr::Variable { name } => {
                if var_binded.contains(name) {
                    HashSet::new()
                } else {
                    HashSet::from([*name])
                }
            }
            Expr::KeyVal { value, .. } => value.free_var(reactive_names, var_binded),
            Expr::Tuple { val } => {
                let mut free_vars = HashSet::new();
                for item in val {
                    free_vars.extend(item.free_var(reactive_names, var_binded));
                }
                free_vars
            }
            Expr::Unop { op: _, expr } => expr.free_var(reactive_names, var_binded),
            Expr::Binop {
                op: _,
                expr1,
                expr2,
            } => {
                let mut free_vars = expr1.free_var(reactive_names, var_binded);
                free_vars.extend(expr2.free_var(reactive_names, var_binded));
                free_vars
            }
            Expr::If { cond, expr1, expr2 } => {
                let mut free_vars = cond.free_var(reactive_names, var_binded);
                free_vars.extend(expr1.free_var(reactive_names, var_binded));
                free_vars.extend(expr2.free_var(reactive_names, var_binded));
                free_vars
            }
            Expr::Func { params, body, .. } => {
                let mut new_binds = var_binded.clone();
                new_binds.extend(params.iter().map(|p| p.name));
                body.free_var(reactive_names, &new_binds)
            }
            Expr::Call { func, args } => {
                let mut free_vars = func.free_var(reactive_names, var_binded);
                for arg in args {
                    free_vars.extend(arg.free_var(reactive_names, var_binded));
                }
                free_vars
            }
            Expr::Action(stmts) => {
                let mut free_vars = HashSet::new();
                for stmt in stmts {
                    match stmt {
                        ActionStmt::Assign { name: _, expr } => {
                            free_vars.extend(expr.free_var(reactive_names, var_binded));
                        }
                        ActionStmt::Do(expr) => {
                            free_vars.extend(expr.free_var(reactive_names, var_binded));
                        }
                        ActionStmt::Assert(expr, _) => {
                            free_vars.extend(expr.free_var(reactive_names, var_binded));
                        }
                        ActionStmt::Let {
                            name: _,
                            ty: _,
                            expr,
                        } => {
                            free_vars.extend(expr.free_var(reactive_names, var_binded));
                        }
                        ActionStmt::Expr(expr) => {
                            free_vars.extend(expr.free_var(reactive_names, var_binded));
                        }
                        ActionStmt::Insert { row, .. } => {
                            free_vars.extend(row.free_var(reactive_names, var_binded));
                        }
                    }
                }
                free_vars.difference(reactive_names).cloned().collect()
            }
            Expr::MemberAccess { .. } => {
                // member access on another service - no local free vars
                HashSet::new()
            }
            Expr::Select {
                table_name,
                where_clause,
                ..
            } => {
                let mut free_vars = where_clause.free_var(reactive_names, var_binded);
                free_vars.insert(*table_name);
                free_vars
            }
            Expr::Fold {
                operation,
                identity,
                ..
            } => {
                let mut free_vars = HashSet::new();
                free_vars.extend(operation.free_var(reactive_names, var_binded));
                free_vars.extend(identity.free_var(reactive_names, var_binded));
                free_vars
            }
        }
    }
}
