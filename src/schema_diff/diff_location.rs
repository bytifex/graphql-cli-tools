#[derive(Debug, Clone, Copy)]
pub enum DiffLocationSegmentType {
    DefaultValue,
    Description,
    Directive,
    DirectiveArgument,
    DirectiveDefinition,
    EnumDefinition,
    EnumValueDefinition,
    Extends,
    Field,
    Implements,
    InputArgument,
    InputObject,
    InterfaceDefinition,
    IsRepeatable,
    Mutation,
    ObjectDefinition,
    ScalarDefinition,
    SchemaDefinition,
    Subscription,
    Query,
    Type,
    TypeDefinition,
    UnionDefinition,
    UnionMemberDefinition,
}

impl DiffLocationSegmentType {
    fn as_str(&self) -> &str {
        match self {
            DiffLocationSegmentType::DefaultValue => "DefaultValue",
            DiffLocationSegmentType::Description => "Description",
            DiffLocationSegmentType::Directive => "Directive",
            DiffLocationSegmentType::DirectiveArgument => "DirectiveArgument",
            DiffLocationSegmentType::DirectiveDefinition => "DirectiveDefinition",
            DiffLocationSegmentType::EnumDefinition => "EnumDefinition",
            DiffLocationSegmentType::EnumValueDefinition => "EnumValueDefinition",
            DiffLocationSegmentType::Extends => "Extends",
            DiffLocationSegmentType::Field => "Field",
            DiffLocationSegmentType::Implements => "Implements",
            DiffLocationSegmentType::InputArgument => "InputArgument",
            DiffLocationSegmentType::InputObject => "InputObject",
            DiffLocationSegmentType::InterfaceDefinition => "InterfaceDefinition",
            DiffLocationSegmentType::IsRepeatable => "IsRepeatable",
            DiffLocationSegmentType::Mutation => "Mutation",
            DiffLocationSegmentType::ObjectDefinition => "ObjectDefinition",
            DiffLocationSegmentType::ScalarDefinition => "ScalarDefinition",
            DiffLocationSegmentType::SchemaDefinition => "SchemaDefinition",
            DiffLocationSegmentType::Subscription => "Subscription",
            DiffLocationSegmentType::Query => "Query",
            DiffLocationSegmentType::Type => "Type",
            DiffLocationSegmentType::TypeDefinition => "TypeDefinition",
            DiffLocationSegmentType::UnionDefinition => "UnionDefinition",
            DiffLocationSegmentType::UnionMemberDefinition => "UnionMemberDefinition",
        }
    }
}

#[derive(Clone)]
struct DiffLocationSegment<'a>(DiffLocationSegmentType, Option<&'a str>);

#[derive(Clone)]
pub struct DiffLocation<'a> {
    segments: Vec<DiffLocationSegment<'a>>,
}

impl<'a> DiffLocation<'a> {
    pub fn new<NameType: Into<Option<&'a str>>>(
        diff_type: DiffLocationSegmentType,
        name: NameType,
    ) -> Self {
        Self {
            segments: vec![DiffLocationSegment::new(diff_type, name)],
        }
    }

    pub fn push<NameType: Into<Option<&'a str>>>(
        &self,
        diff_type: DiffLocationSegmentType,
        name: NameType,
    ) -> Self {
        let mut ret = self.clone();
        ret.segments.push(DiffLocationSegment::new(diff_type, name));
        ret
    }
}

impl<'a> DiffLocationSegment<'a> {
    fn new<NameType: Into<Option<&'a str>>>(
        diff_type: DiffLocationSegmentType,
        name: NameType,
    ) -> Self {
        Self(diff_type, name.into())
    }
}

impl<'a> std::fmt::Display for DiffLocationSegment<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.1 {
            Some(name) => write!(f, "{}({})", self.0.as_str(), name),
            None => write!(f, "{}", self.0.as_str()),
        }
    }
}

impl<'a> std::fmt::Display for DiffLocation<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut buffer = String::new();
        for (count, segment) in self.segments.iter().enumerate() {
            if count != 0 {
                buffer += format!(" -> {}", segment).as_str();
            } else {
                buffer += format!("{}", segment).as_str();
            }
        }

        write!(f, "{}", buffer)
    }
}
