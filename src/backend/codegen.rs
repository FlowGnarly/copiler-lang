use crate::frontend::parser::{Expression, Statement};

pub enum GenType {
    VariableDeclaration {
        local: bool,
        ident: String,
        value: String,
        value_type: Option<String>,
    },
    FunctionCall {
        ident: String,
        values: Vec<String>,
    },
    FunctionBody {
        local: bool,
        ident: String,
        params: Vec<String>,
    },
    ClassConstructor {
        ident: String,
        props: String,
        methods: Vec<Box<GenType>>,
    },
    Return {
        value: String,
    },
    LScope,
    RScope,
}

pub struct CodeGen {
    pub src: String,
    root_stmt: Statement,
    nest: usize,
}

impl CodeGen {
    pub fn new(stmt: Statement) -> Self {
        Self {
            src: String::new(),
            root_stmt: stmt,
            nest: 0,
        }
    }

    fn write(&mut self, code: GenType) {
        let code: String = match code {
            GenType::LScope => "do".into(),
            GenType::RScope => "end".into(),
            GenType::VariableDeclaration {
                local,
                ident,
                value,
                value_type,
            } =>
            {
                #[allow(clippy::collapsible_else_if)]
                if local {
                    if let Some(value_type) = value_type {
                        format!("local {ident}: {value_type} = {value}")
                    } else {
                        format!("local {ident} = {value}")
                    }
                } else {
                    if let Some(value_type) = value_type {
                        format!("{ident} = {value} :: {value_type}")
                    } else {
                        format!("{ident} = {value}")
                    }
                }
            }
            GenType::FunctionCall { ident, values } => {
                let values_str = if !values.is_empty() {
                    let mut values_str = values[0].clone();

                    for value in values.iter().skip(1) {
                        values_str += &format!(", {value}")
                    }

                    values_str
                } else {
                    String::new()
                };

                format!("{ident}({values_str})")
            }
            GenType::FunctionBody {
                local,
                ident,
                params,
            } => {
                let params_str = if !params.is_empty() {
                    let mut params_str = params[0].clone();

                    for param in params.iter().skip(1) {
                        params_str += &format!(", {param}")
                    }

                    params_str
                } else {
                    String::new()
                };

                if local {
                    format!("local function {ident}({params_str})")
                } else {
                    format!("function {ident}({params_str})")
                }
            }
            GenType::ClassConstructor {
                ident,
                props: _,
                methods: _,
            } => {
                format!("local {ident} = {{}}")
            }
            GenType::Return { value } => format!("return {value}"),
        };

        let spaces = "    ".repeat(self.nest);
        self.src += &format!("{spaces}{code}\n");
    }

    fn expr_to_value(expr: Expression) -> String {
        match expr {
            Expression::Identifier(ident) => ident,
            Expression::Char(char) => format!("\"{char}\""),
            Expression::String(string) => format!("\"{string}\""),
            Expression::Number(number) => number.to_string(),
            Expression::Indexing(l, r) => {
                format!("{}.{}", Self::expr_to_value(*l), Self::expr_to_value(*r))
            }
            _ => panic!(),
        }
    }

    fn expr_to_value_with_type(&self, expr: Expression) -> (Option<String>, String) {
        let type_str: Option<String> = match expr.clone() {
            Expression::Identifier(_) => None,
            Expression::Indexing(_, _) => None,
            Expression::Function { .. } => None,
            Expression::Parameter { expected_type, .. } => {
                (*expected_type).map(Self::expr_to_value)
            }
            Expression::Char(_) | Expression::String(_) => Some("string".into()),
            Expression::Number(_) => Some("number".into()),
            Expression::ClassBody { .. } => todo!(),
        };

        let value_str = match expr {
            Expression::Identifier(ident) => ident,
            Expression::Parameter { ident, .. } => Self::expr_to_value(*ident),
            Expression::Char(char) => format!("\"{char}\""),
            Expression::String(string) => format!("\"{string}\""),
            Expression::Number(number) => number.to_string(),
            _ => panic!(),
        };

        (type_str, value_str)
    }

    fn indexing_to_value(l: Expression, r: Expression) -> String {
        format!("{}.{}", Self::expr_to_value(l), Self::expr_to_value(r))
    }

    #[allow(clippy::only_used_in_recursion)]
    fn gen_statement(&mut self, stmt: Statement) {
        match stmt {
            Statement::Scope(statements) => {
                self.write(GenType::LScope);
                self.nest += 1;

                for stmt in statements {
                    self.gen_statement(stmt.clone());
                }

                self.nest -= 1;
                self.write(GenType::RScope);
            }
            Statement::ClassConstructor { ident, body } => {
                //
                let ident_str = Self::expr_to_value(ident.clone());

                self.write(GenType::VariableDeclaration {
                    local: true,
                    ident: ident_str.clone(),
                    value: "{}".into(),
                    value_type: None,
                });

                self.write(GenType::FunctionBody {
                    local: false,
                    ident: format!("{ident_str}.new"),
                    params: vec![],
                });

                self.nest += 1;

                self.write(GenType::VariableDeclaration {
                    local: true,
                    ident: String::from("self"),
                    value: String::from("{}"),
                    value_type: None,
                });

                if let Expression::ClassBody { properties } = body {
                    for prop in properties {
                        if let Statement::VariableDeclaration { ident, value } = prop {
                            let (value_type_str, value_str) = self.expr_to_value_with_type(value);

                            self.write(GenType::VariableDeclaration {
                                local: false,
                                ident: match ident {
                                    Expression::Identifier(ident) => format!("self.{ident}"),
                                    Expression::Indexing(l, r) => Self::indexing_to_value(*l, *r),
                                    _ => panic!("{ident:?} can't be converted to identifier"),
                                },
                                value: value_str,
                                value_type: value_type_str,
                            })
                        } else {
                            todo!()
                        }
                    }
                } else {
                    panic!("{body:?} is not a valid class body")
                }

                self.write(GenType::Return {
                    value: String::from("self"),
                });

                self.nest -= 1;

                self.write(GenType::RScope);
            }
            Statement::VariableDeclaration { ident, value } => {
                if let Expression::Identifier(value_ident) = value.clone() {
                    self.write(GenType::VariableDeclaration {
                        local: true,
                        ident: match ident {
                            Expression::Identifier(ident) => ident,
                            Expression::Indexing(l, r) => Self::indexing_to_value(*l, *r),
                            _ => panic!("{ident:?} can't be converted to identifier"),
                        },
                        value: value_ident.to_owned(),
                        value_type: None,
                    });
                } else if let Expression::Function { params, stmt } = value {
                    let mut params_str = vec![];

                    let param_str = self.expr_to_value_with_type(params[0].clone());
                    params_str.push(if let Some(expected_type) = param_str.0 {
                        format!("{}: {expected_type}", param_str.1)
                    } else {
                        param_str.1
                    });

                    for param in params.iter().skip(1) {
                        let param_str = self.expr_to_value_with_type(param.clone());
                        params_str.push(if let Some(expected_type) = param_str.0 {
                            format!("{}: {expected_type}", param_str.1)
                        } else {
                            param_str.1
                        });
                    }

                    self.write(GenType::FunctionBody {
                        local: true,
                        ident: Self::expr_to_value(ident),
                        params: params_str,
                    });

                    if let Statement::Scope(scope) = *stmt {
                        self.nest += 1;

                        for stmt in scope {
                            self.gen_statement(stmt.clone());
                        }

                        self.nest -= 1;
                    } else {
                        self.gen_statement(*stmt);
                    }

                    self.write(GenType::RScope);
                } else {
                    let (type_str, value_str) = self.expr_to_value_with_type(value);

                    self.write(GenType::VariableDeclaration {
                        local: true,
                        ident: match ident {
                            Expression::Identifier(ident) => ident,
                            Expression::Indexing(l, r) => Self::indexing_to_value(*l, *r),
                            _ => panic!("{ident:?} can't be converted to identifier"),
                        },
                        value: value_str,
                        value_type: type_str,
                    });
                }
            }
            Statement::FunctionCall { ident, args } => self.write(GenType::FunctionCall {
                ident: match ident {
                    Expression::Identifier(ident) => ident,
                    Expression::Indexing(l, r) => Self::indexing_to_value(*l, *r),
                    _ => panic!("{ident:?} can't be converted to identifier"),
                },
                values: args
                    .iter()
                    .map(|expr| {
                        let (_, value_str) = self.expr_to_value_with_type(expr.clone());
                        value_str
                    })
                    .collect(),
            }),
        }
    }

    pub fn run(&mut self) {
        if let Statement::Scope(scope) = self.root_stmt.clone() {
            for stmt in scope {
                self.gen_statement(stmt.clone());
            }
        } else {
            panic!("Root stmt must be a scope");
        }
    }
}
