use async_graphql::{Name, Positioned, Value};
use async_graphql_parser::types::{
    ConstDirective, DirectiveDefinition, EnumValueDefinition, FieldDefinition,
    InputValueDefinition, SchemaDefinition, TypeDefinition,
};

pub trait Named {
    fn name(&self) -> &str;
}

impl Named for Name {
    fn name(&self) -> &str {
        self.as_str()
    }
}

impl Named for DirectiveDefinition {
    fn name(&self) -> &str {
        self.name.node.as_str()
    }
}

impl Named for SchemaDefinition {
    fn name(&self) -> &str {
        "schema"
    }
}

impl Named for TypeDefinition {
    fn name(&self) -> &str {
        self.name.node.as_str()
    }
}

impl Named for ConstDirective {
    fn name(&self) -> &str {
        self.name.node.as_str()
    }
}

impl Named for InputValueDefinition {
    fn name(&self) -> &str {
        self.name.node.as_str()
    }
}

impl Named for FieldDefinition {
    fn name(&self) -> &str {
        self.name.node.as_str()
    }
}

impl Named for EnumValueDefinition {
    fn name(&self) -> &str {
        self.value.node.as_str()
    }
}

impl Named for (Positioned<Name>, Positioned<Value>) {
    fn name(&self) -> &str {
        self.0.node.as_str()
    }
}
