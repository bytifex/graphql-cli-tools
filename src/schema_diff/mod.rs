mod diff_location;
mod named;
mod try_into_service_document;

use std::fmt::Display;

use async_graphql::{Name, Positioned, Value};
use async_graphql_parser::types::{
    BaseType, ConstDirective, DirectiveDefinition, EnumType, EnumValueDefinition, FieldDefinition,
    InputObjectType, InputValueDefinition, InterfaceType, ObjectType, SchemaDefinition,
    ServiceDocument, Type, TypeDefinition, TypeKind, TypeSystemDefinition, UnionType,
};
use diff_location::{DiffLocation, DiffLocationSegmentType};
use named::Named;
use try_into_service_document::TryIntoServiceDocument;

pub enum ChangeType {
    Breaking,
    NonBreaking,
    Unknown,
}

impl std::fmt::Display for ChangeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChangeType::Breaking => write!(f, "breaking"),
            ChangeType::NonBreaking => write!(f, "non-breaking"),
            ChangeType::Unknown => write!(f, "unknown"),
        }
    }
}

pub fn diff_schema(
    schema_left: impl TryIntoServiceDocument<Error: std::error::Error>,
    schema_right: impl TryIntoServiceDocument<Error: std::error::Error>,
) -> Result<(), Box<dyn std::error::Error>> {
    let schema_left = schema_left.try_into_service_document()?;
    let schema_right = schema_right.try_into_service_document()?;

    compare_iterators(
        DiffLocation::new(DiffLocationSegmentType::DirectiveDefinition, None),
        || filter_directive_definitions_of_service_document(&schema_left),
        || filter_directive_definitions_of_service_document(&schema_right),
        |_new_item| ChangeType::NonBreaking,
        compare_directive_definitions,
    );

    compare_iterators(
        DiffLocation::new(DiffLocationSegmentType::SchemaDefinition, None),
        || filter_schemas_of_service_document(&schema_left),
        || filter_schemas_of_service_document(&schema_right),
        |_new_item| ChangeType::Breaking,
        compare_schema_definitions,
    );

    compare_iterators(
        DiffLocation::new(DiffLocationSegmentType::TypeDefinition, None),
        || filter_types_of_service_document(&schema_left),
        || filter_types_of_service_document(&schema_right),
        |_new_item| ChangeType::NonBreaking,
        compare_type_definitions,
    );

    Ok(())
}

fn filter_directive_definitions_of_service_document(
    service_document: &ServiceDocument,
) -> impl Iterator<Item = &DirectiveDefinition> {
    service_document
        .definitions
        .iter()
        .filter_map(|definition| match definition {
            TypeSystemDefinition::Directive(directive) => Some(&directive.node),
            TypeSystemDefinition::Schema(_schema) => None,
            TypeSystemDefinition::Type(r#_type) => None,
        })
}

fn filter_schemas_of_service_document(
    service_document: &ServiceDocument,
) -> impl Iterator<Item = &SchemaDefinition> {
    service_document
        .definitions
        .iter()
        .filter_map(|definition| match definition {
            TypeSystemDefinition::Directive(_directive) => None,
            TypeSystemDefinition::Schema(schema) => Some(&schema.node),
            TypeSystemDefinition::Type(r#_type) => None,
        })
}

fn filter_types_of_service_document(
    service_document: &ServiceDocument,
) -> impl Iterator<Item = &TypeDefinition> {
    service_document
        .definitions
        .iter()
        .filter_map(|definition| match definition {
            TypeSystemDefinition::Directive(_directive) => None,
            TypeSystemDefinition::Schema(_schema) => None,
            TypeSystemDefinition::Type(r#type) => Some(&r#type.node),
        })
}

fn compare_iterators<
    'a,
    'b,
    T: Named + 'static,
    LeftIteratorType: Iterator<Item = &'a T>,
    RightIteratorType: Iterator<Item = &'b T>,
>(
    diff_location: DiffLocation,
    left_iter_generator: impl Fn() -> LeftIteratorType,
    right_iter_generator: impl Fn() -> RightIteratorType,
    change_type_if_added: impl Fn(&T) -> ChangeType,
    item_comparator_fn: impl Fn(&T, &T),
) {
    // item is added
    // non-breaking changes
    for right in right_iter_generator() {
        match left_iter_generator().find(|left| left.name() == right.name()) {
            Some(left) => item_comparator_fn(left, right),
            None => {
                println!(
                    "{}: item is added to the right, name = '{}' (breaking = {})",
                    diff_location,
                    right.name(),
                    change_type_if_added(right),
                );
            }
        }
    }

    // item is removed
    // breaking changes
    for left in left_iter_generator() {
        match right_iter_generator().find(|right| right.name() == left.name()) {
            Some(_right) => {
                // already compared
            }
            None => {
                println!(
                    "{}: item is removed, name = '{}' (breaking = {})",
                    diff_location,
                    left.name(),
                    ChangeType::Breaking,
                );
            }
        }
    }
}

fn compare_directive_definitions(
    definition_left: &DirectiveDefinition,
    definition_right: &DirectiveDefinition,
) {
    let DirectiveDefinition {
        description: description_left,
        name: name_left,
        arguments: arguments_left,
        is_repeatable: is_repeatable_left,
        locations: _locations_left,
    } = definition_left;

    let DirectiveDefinition {
        description: description_right,
        name: name_right,
        arguments: arguments_right,
        is_repeatable: is_repeatable_right,
        locations: _locations_right,
    } = definition_right;

    assert_eq!(name_left.node.as_str(), name_right.node.as_str());

    let diff_location = DiffLocation::new(
        DiffLocationSegmentType::DirectiveDefinition,
        Some(name_right.node.as_str()),
    );

    compare_optional_strings(
        diff_location.push(DiffLocationSegmentType::Description, None),
        ChangeType::NonBreaking,
        ChangeType::NonBreaking,
        ChangeType::NonBreaking,
        false,
        description_left
            .as_ref()
            .map(|positioned| positioned.node.as_str()),
        description_right
            .as_ref()
            .map(|positioned| positioned.node.as_str()),
    );

    compare_iterators(
        diff_location.push(DiffLocationSegmentType::InputArgument, None),
        || arguments_left.iter().map(|positioned| &positioned.node),
        || arguments_right.iter().map(|positioned| &positioned.node),
        |new_item| {
            if new_item.default_value.is_some() || new_item.ty.node.nullable {
                ChangeType::NonBreaking
            } else {
                ChangeType::Breaking
            }
        },
        |left, right| {
            compare_input_value_definitions(
                diff_location.push(DiffLocationSegmentType::InputArgument, Some(right.name())),
                left,
                right,
            )
        },
    );

    compare_comparables(
        diff_location.push(DiffLocationSegmentType::IsRepeatable, None),
        is_repeatable_left,
        is_repeatable_right,
        if *is_repeatable_right {
            ChangeType::NonBreaking
        } else {
            ChangeType::Breaking
        },
    );
}

fn compare_schema_definitions(
    definition_left: &SchemaDefinition,
    definition_right: &SchemaDefinition,
) {
    let SchemaDefinition {
        extend: extend_left,
        directives: directives_left,
        query: query_left,
        mutation: mutation_left,
        subscription: subscription_left,
    } = definition_left;

    let SchemaDefinition {
        extend: extend_right,
        directives: directives_right,
        query: query_right,
        mutation: mutation_right,
        subscription: subscription_right,
    } = definition_right;

    let diff_location = DiffLocation::new(
        DiffLocationSegmentType::SchemaDefinition,
        Some(definition_right.name()),
    );

    compare_comparables(
        diff_location.push(DiffLocationSegmentType::Extends, None),
        extend_left,
        extend_right,
        ChangeType::NonBreaking,
    );

    compare_iterators(
        diff_location.push(DiffLocationSegmentType::Directive, None),
        || directives_left.iter().map(|positioned| &positioned.node),
        || directives_right.iter().map(|positioned| &positioned.node),
        |_new_item| ChangeType::Unknown,
        |left, right| {
            compare_const_directives(
                diff_location
                    .push(DiffLocationSegmentType::Directive, Some(right.name()))
                    .clone(),
                left,
                right,
            )
        },
    );

    compare_optional_strings(
        diff_location.push(DiffLocationSegmentType::Query, None),
        ChangeType::Breaking,
        ChangeType::NonBreaking,
        ChangeType::Breaking,
        true,
        query_left
            .as_ref()
            .map(|positioned| positioned.node.as_str()),
        query_right
            .as_ref()
            .map(|positioned| positioned.node.as_str()),
    );

    compare_optional_strings(
        diff_location.push(DiffLocationSegmentType::Mutation, None),
        ChangeType::Breaking,
        ChangeType::NonBreaking,
        ChangeType::Breaking,
        true,
        mutation_left
            .as_ref()
            .map(|positioned| positioned.node.as_str()),
        mutation_right
            .as_ref()
            .map(|positioned| positioned.node.as_str()),
    );

    compare_optional_strings(
        diff_location.push(DiffLocationSegmentType::Subscription, None),
        ChangeType::Breaking,
        ChangeType::NonBreaking,
        ChangeType::Breaking,
        true,
        subscription_left
            .as_ref()
            .map(|positioned| positioned.node.as_str()),
        subscription_right
            .as_ref()
            .map(|positioned| positioned.node.as_str()),
    );
}

fn compare_type_definitions(definition_left: &TypeDefinition, definition_right: &TypeDefinition) {
    let TypeDefinition {
        extend: extend_left,
        description: description_left,
        name: name_left,
        directives: directives_left,
        kind: kind_left,
    } = definition_left;

    let TypeDefinition {
        extend: extend_right,
        description: description_right,
        name: name_right,
        directives: directives_right,
        kind: kind_right,
    } = definition_right;

    assert_eq!(name_left.node.as_str(), name_right.node.as_str());
    let name = name_right.node.as_str();

    let mut diff_location = DiffLocation::new(DiffLocationSegmentType::TypeDefinition, Some(name));

    match (kind_left, kind_right) {
        (TypeKind::Scalar, TypeKind::Scalar) => {
            // since the name of the scalars are the same, therefore there is no need to compare
            diff_location =
                DiffLocation::new(DiffLocationSegmentType::ScalarDefinition, Some(name));
        }
        (TypeKind::Object(type_left), TypeKind::Object(type_right)) => {
            diff_location =
                DiffLocation::new(DiffLocationSegmentType::ObjectDefinition, Some(name));
            compare_object_types(diff_location.clone(), type_left, type_right)
        }
        (TypeKind::Interface(type_left), TypeKind::Interface(type_right)) => {
            diff_location =
                DiffLocation::new(DiffLocationSegmentType::InterfaceDefinition, Some(name));
            compare_interface_types(diff_location.clone(), type_left, type_right)
        }
        (TypeKind::Union(type_left), TypeKind::Union(type_right)) => {
            diff_location = DiffLocation::new(DiffLocationSegmentType::UnionDefinition, Some(name));
            compare_union_types(diff_location.clone(), type_left, type_right)
        }
        (TypeKind::Enum(type_left), TypeKind::Enum(type_right)) => {
            diff_location = DiffLocation::new(DiffLocationSegmentType::EnumDefinition, Some(name));
            compare_enum_types(diff_location.clone(), type_left, type_right)
        }
        (TypeKind::InputObject(type_left), TypeKind::InputObject(type_right)) => {
            diff_location = DiffLocation::new(DiffLocationSegmentType::InputObject, Some(name));
            compare_input_object_types(diff_location.clone(), type_left, type_right)
        }
        // comparing types of different kinds, the code will fail to build this way if a new type is introduced
        (TypeKind::Scalar, _type_kind_right)
        | (TypeKind::Object(_), _type_kind_right)
        | (TypeKind::Interface(_), _type_kind_right)
        | (TypeKind::Union(_), _type_kind_right)
        | (TypeKind::Enum(_), _type_kind_right)
        | (TypeKind::InputObject(_), _type_kind_right) => {
            println!(
                "{}: type mismatch, name = '{}' (breaking = {})",
                diff_location,
                name,
                ChangeType::Breaking,
            );
        }
    }

    compare_comparables(
        diff_location.push(DiffLocationSegmentType::Extends, None),
        extend_left,
        extend_right,
        ChangeType::NonBreaking,
    );

    compare_optional_strings(
        diff_location.push(DiffLocationSegmentType::Description, None),
        ChangeType::NonBreaking,
        ChangeType::NonBreaking,
        ChangeType::NonBreaking,
        false,
        description_left
            .as_ref()
            .map(|positioned| positioned.node.as_str()),
        description_right
            .as_ref()
            .map(|positioned| positioned.node.as_str()),
    );

    compare_iterators(
        diff_location.push(DiffLocationSegmentType::Directive, None),
        || directives_left.iter().map(|positioned| &positioned.node),
        || directives_right.iter().map(|positioned| &positioned.node),
        |_new_item| ChangeType::Unknown,
        |left, right| {
            compare_const_directives(
                diff_location.push(DiffLocationSegmentType::Directive, Some(right.name())),
                left,
                right,
            )
        },
    );
}

fn compare_optional_strings(
    diff_location: DiffLocation,
    change_type_if_different: ChangeType,
    change_type_if_added: ChangeType,
    change_type_if_removed: ChangeType,
    print_string: bool,
    left: Option<&str>,
    right: Option<&str>,
) {
    match (left, right) {
        (Some(mut left), Some(mut right)) => {
            if left != right {
                if !print_string {
                    left = "?";
                    right = "?";
                }

                println!(
                    "{}: left value = '{}', right value = '{}' (breaking = {})",
                    diff_location, left, right, change_type_if_different,
                );
            }
        }
        (None, None) => (),
        // breaking change
        (Some(mut left), None) => {
            if !print_string {
                left = "?";
            }

            println!(
                "{}: item is removed, value = '{}' (breaking = {})",
                diff_location, left, change_type_if_removed,
            );
        }
        // non-breaking change
        (None, Some(mut right)) => {
            if !print_string {
                right = "?";
            }

            println!(
                "{}: added to right, value = '{}' (breaking = {})",
                diff_location, right, change_type_if_added,
            );
        }
    }
}

fn compare_object_types(
    diff_location: DiffLocation,
    type_left: &ObjectType,
    type_right: &ObjectType,
) {
    let ObjectType {
        implements: implements_left,
        fields: fields_left,
    } = type_left;

    let ObjectType {
        implements: implements_right,
        fields: fields_right,
    } = type_right;

    compare_iterators(
        diff_location.push(DiffLocationSegmentType::Implements, None),
        || implements_left.iter().map(|positioned| &positioned.node),
        || implements_right.iter().map(|positioned| &positioned.node),
        |_new_item| ChangeType::NonBreaking,
        |_left, _right| {
            // since this closure is called only if the names are the same, therofe the items are the same
        },
    );

    compare_iterators(
        diff_location.push(DiffLocationSegmentType::Field, None),
        || fields_left.iter().map(|positioned| &positioned.node),
        || fields_right.iter().map(|positioned| &positioned.node),
        |_new_item| ChangeType::NonBreaking,
        |left, right| {
            compare_field_definitions(
                diff_location.push(DiffLocationSegmentType::Field, Some(left.name())),
                left,
                right,
            )
        },
    );
}

fn compare_interface_types(
    diff_location: DiffLocation,
    type_left: &InterfaceType,
    type_right: &InterfaceType,
) {
    let InterfaceType {
        implements: implements_left,
        fields: fields_left,
    } = type_left;

    let InterfaceType {
        implements: implements_right,
        fields: fields_right,
    } = type_right;

    compare_iterators(
        diff_location.push(DiffLocationSegmentType::Implements, None),
        || implements_left.iter().map(|positioned| &positioned.node),
        || implements_right.iter().map(|positioned| &positioned.node),
        |_new_item| ChangeType::NonBreaking,
        |_left, _right| {
            // since this closure is called only if the names are the same, therofe the items are the same
        },
    );

    compare_iterators(
        diff_location.push(DiffLocationSegmentType::Field, None),
        || fields_left.iter().map(|positioned| &positioned.node),
        || fields_right.iter().map(|positioned| &positioned.node),
        |_new_item| ChangeType::NonBreaking,
        |left, right| {
            compare_field_definitions(
                diff_location.push(DiffLocationSegmentType::Field, Some(right.name())),
                left,
                right,
            )
        },
    );
}

fn compare_union_types(diff_location: DiffLocation, type_left: &UnionType, type_right: &UnionType) {
    let UnionType {
        members: members_left,
    } = type_left;

    let UnionType {
        members: members_right,
    } = type_right;

    compare_iterators(
        diff_location.push(DiffLocationSegmentType::UnionMemberDefinition, None),
        || members_left.iter().map(|positioned| &positioned.node),
        || members_right.iter().map(|positioned| &positioned.node),
        |_new_item| ChangeType::Breaking,
        |_left, _right| {
            // since this closure is called only if the names are the same, therofe the items are the same
        },
    );
}

fn compare_enum_types(diff_location: DiffLocation, type_left: &EnumType, type_right: &EnumType) {
    let EnumType {
        values: values_left,
    } = type_left;

    let EnumType {
        values: values_right,
    } = type_right;

    compare_iterators(
        diff_location.push(DiffLocationSegmentType::EnumValueDefinition, None),
        || values_left.iter().map(|positioned| &positioned.node),
        || values_right.iter().map(|positioned| &positioned.node),
        |_new_item| ChangeType::NonBreaking,
        |left, right| {
            compare_enum_value_definitions(
                diff_location.push(
                    DiffLocationSegmentType::EnumValueDefinition,
                    Some(right.name()),
                ),
                left,
                right,
            )
        },
    );
}

fn compare_input_object_types(
    diff_location: DiffLocation,
    type_left: &InputObjectType,
    type_right: &InputObjectType,
) {
    let InputObjectType {
        fields: fields_left,
    } = type_left;

    let InputObjectType {
        fields: fields_right,
    } = type_right;

    compare_iterators(
        diff_location.push(DiffLocationSegmentType::Field, None),
        || fields_left.iter().map(|positioned| &positioned.node),
        || fields_right.iter().map(|positioned| &positioned.node),
        |new_item| {
            if new_item.default_value.is_some() || new_item.ty.node.nullable {
                ChangeType::NonBreaking
            } else {
                ChangeType::Breaking
            }
        },
        |left, right| {
            compare_input_value_definitions(
                diff_location.push(DiffLocationSegmentType::Field, Some(right.name())),
                left,
                right,
            )
        },
    );
}

fn compare_field_definitions(
    diff_location: DiffLocation,
    definition_left: &FieldDefinition,
    definition_right: &FieldDefinition,
) {
    let FieldDefinition {
        description: description_left,
        name: name_left,
        arguments: arguments_left,
        ty: ty_left,
        directives: directives_left,
    } = definition_left;

    let FieldDefinition {
        description: description_right,
        name: name_right,
        arguments: arguments_right,
        ty: ty_right,
        directives: directives_right,
    } = definition_right;

    assert_eq!(name_left.node.as_str(), name_right.node.as_str());

    compare_types(
        diff_location.push(DiffLocationSegmentType::Type, None),
        &ty_left.node,
        &ty_right.node,
        false,
    );

    compare_optional_strings(
        diff_location.push(DiffLocationSegmentType::Description, None),
        ChangeType::NonBreaking,
        ChangeType::NonBreaking,
        ChangeType::NonBreaking,
        false,
        description_left
            .as_ref()
            .map(|positioned| positioned.node.as_str()),
        description_right
            .as_ref()
            .map(|positioned| positioned.node.as_str()),
    );

    compare_iterators(
        diff_location.push(DiffLocationSegmentType::InputArgument, None),
        || arguments_left.iter().map(|positioned| &positioned.node),
        || arguments_right.iter().map(|positioned| &positioned.node),
        |new_item| {
            if new_item.default_value.is_some() || new_item.ty.node.nullable {
                ChangeType::NonBreaking
            } else {
                ChangeType::Breaking
            }
        },
        |left, right| {
            compare_input_value_definitions(
                diff_location.push(DiffLocationSegmentType::InputArgument, Some(right.name())),
                left,
                right,
            )
        },
    );

    compare_iterators(
        diff_location.push(DiffLocationSegmentType::Directive, None),
        || directives_left.iter().map(|positioned| &positioned.node),
        || directives_right.iter().map(|positioned| &positioned.node),
        |_new_item| ChangeType::Unknown,
        |left, right| {
            compare_const_directives(
                diff_location.push(DiffLocationSegmentType::Directive, Some(right.name())),
                left,
                right,
            )
        },
    );
}

fn compare_const_directives(
    diff_location: DiffLocation,
    directive_left: &ConstDirective,
    directive_right: &ConstDirective,
) {
    let ConstDirective {
        name: name_left,
        arguments: arguments_left,
    } = directive_left;

    let ConstDirective {
        name: name_right,
        arguments: arguments_right,
    } = directive_right;

    assert_eq!(name_left.node.as_str(), name_right.node.as_str());

    compare_iterators(
        diff_location.push(DiffLocationSegmentType::DirectiveArgument, None),
        || {
            arguments_left.iter()
            // .map(|(name, value)| &(&name.node, &value.node))
        },
        || {
            arguments_right.iter()
            // .map(|(name, value)| &(&name.node, &value.node))
        },
        |_new_item| ChangeType::Unknown,
        |left, right| {
            compare_const_directive_argument_value(
                diff_location.push(
                    DiffLocationSegmentType::DirectiveArgument,
                    Some(right.name()),
                ),
                left,
                right,
            )
        },
    );
}

fn compare_enum_value_definitions(
    diff_location: DiffLocation,
    definition_left: &EnumValueDefinition,
    definition_right: &EnumValueDefinition,
) {
    let EnumValueDefinition {
        description: description_left,
        value: value_left,
        directives: directives_left,
    } = definition_left;

    let EnumValueDefinition {
        description: description_right,
        value: value_right,
        directives: directives_right,
    } = definition_right;

    assert_eq!(value_left.node.as_str(), value_right.node.as_str());

    compare_optional_strings(
        diff_location.push(DiffLocationSegmentType::Description, None),
        ChangeType::NonBreaking,
        ChangeType::NonBreaking,
        ChangeType::NonBreaking,
        false,
        description_left
            .as_ref()
            .map(|positioned| positioned.node.as_str()),
        description_right
            .as_ref()
            .map(|positioned| positioned.node.as_str()),
    );

    compare_iterators(
        diff_location.push(DiffLocationSegmentType::Directive, None),
        || directives_left.iter().map(|positioned| &positioned.node),
        || directives_right.iter().map(|positioned| &positioned.node),
        |_new_item| ChangeType::Unknown,
        |left, right| {
            compare_const_directives(
                diff_location.push(DiffLocationSegmentType::Directive, Some(right.name())),
                left,
                right,
            )
        },
    );
}

fn compare_input_value_definitions(
    diff_location: DiffLocation,
    definition_left: &InputValueDefinition,
    definition_right: &InputValueDefinition,
) {
    let InputValueDefinition {
        description: description_left,
        name: name_left,
        ty: ty_left,
        default_value: default_value_left,
        directives: directives_left,
    } = definition_left;

    let InputValueDefinition {
        description: description_right,
        name: name_right,
        ty: ty_right,
        default_value: default_value_right,
        directives: directives_right,
    } = definition_right;

    assert_eq!(name_left.node.as_str(), name_right.node.as_str());

    compare_types(
        diff_location.push(DiffLocationSegmentType::Type, None),
        &ty_left.node,
        &ty_right.node,
        true,
    );

    {
        let diff_location = diff_location.push(DiffLocationSegmentType::DefaultValue, None);

        match (
            default_value_left.as_ref().map(|value| &value.node),
            default_value_right.as_ref().map(|value| &value.node),
        ) {
            (Some(left), Some(right)) => {
                if left != right {
                    println!(
                        "{}: left value = '{}', right value = '{}' (breaking = {})",
                        diff_location,
                        left,
                        right,
                        ChangeType::Breaking,
                    );
                }
            }
            (None, None) => (),
            // breaking change
            (Some(left), None) => {
                println!(
                    "{}: item is removed, value = '{}' (breaking = {})",
                    diff_location,
                    left,
                    ChangeType::Breaking,
                );
            }
            // non-breaking change
            (None, Some(right)) => {
                println!(
                    "{}: added to right, value = '{}' (breaking = {})",
                    diff_location,
                    right,
                    ChangeType::NonBreaking,
                );
            }
        }
    }

    compare_optional_strings(
        diff_location.push(DiffLocationSegmentType::Description, None),
        ChangeType::NonBreaking,
        ChangeType::NonBreaking,
        ChangeType::NonBreaking,
        false,
        description_left
            .as_ref()
            .map(|positioned| positioned.node.as_str()),
        description_right
            .as_ref()
            .map(|positioned| positioned.node.as_str()),
    );

    compare_iterators(
        diff_location.push(DiffLocationSegmentType::Directive, None),
        || directives_left.iter().map(|positioned| &positioned.node),
        || directives_right.iter().map(|positioned| &positioned.node),
        |_new_item| ChangeType::Unknown,
        |left, right| {
            compare_const_directives(
                diff_location.push(DiffLocationSegmentType::Directive, Some(right.name())),
                left,
                right,
            )
        },
    );
}

fn compare_const_directive_argument_value(
    diff_location: DiffLocation,
    arg_left: &(Positioned<Name>, Positioned<Value>),
    arg_right: &(Positioned<Name>, Positioned<Value>),
) {
    compare_comparables(
        diff_location.push(DiffLocationSegmentType::DirectiveArgument, None),
        &arg_left.1.node,
        &arg_right.1.node,
        ChangeType::Unknown,
    );
}

fn compare_comparables<T: Display + Eq + PartialEq + ?Sized>(
    diff_location: DiffLocation,
    left: &T,
    right: &T,
    change_type: ChangeType,
) {
    if *left != *right {
        println!(
            "{}: left value = {}, right value = {} (breaking = {})",
            diff_location, left, right, change_type,
        )
    }
}

fn compare_types_recursive(left: &Type, right: &Type, is_argument: bool) -> Option<ChangeType> {
    #[allow(clippy::collapsible_else_if)]
    let change_type = if is_argument {
        if !left.nullable && right.nullable {
            Some(ChangeType::NonBreaking)
        } else if left.nullable && !right.nullable {
            Some(ChangeType::Breaking)
        } else {
            None
        }
    } else {
        if !left.nullable && right.nullable {
            Some(ChangeType::Breaking)
        } else if left.nullable && !right.nullable {
            Some(ChangeType::NonBreaking)
        } else {
            None
        }
    };

    if change_type.is_some() {
        return change_type;
    }

    match (&left.base, &right.base) {
        (BaseType::Named(left_name), BaseType::Named(right_name)) => {
            if left_name == right_name {
                if left.nullable == right.nullable {
                    None
                } else if left.nullable {
                    Some(ChangeType::Breaking)
                } else {
                    Some(ChangeType::NonBreaking)
                }
            } else {
                Some(ChangeType::Breaking)
            }
        }
        (BaseType::List(left_inner), BaseType::List(right_inner)) => {
            compare_types_recursive(left_inner, right_inner, is_argument)
        }
        _ => Some(ChangeType::Breaking),
    }
}

fn compare_types(diff_location: DiffLocation, left: &Type, right: &Type, is_argument: bool) {
    if let Some(change_type) = compare_types_recursive(left, right, is_argument) {
        println!(
            "{}: left value = {}, right value = {} (breaking = {})",
            diff_location, left, right, change_type,
        )
    }
}
