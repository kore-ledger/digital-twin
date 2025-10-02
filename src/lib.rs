use kore_contract_sdk as sdk;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet};

fn has_cycle(
    node: &str,
    graph: &HashMap<String, Vec<String>>,
    visited: &mut HashSet<String>,
    stack: &mut HashSet<String>,
) -> bool {
    if stack.contains(node) {
        return true;
    }
    if visited.contains(node) {
        return false;
    }

    visited.insert(node.to_string());
    stack.insert(node.to_string());

    if let Some(neighbors) = graph.get(node) {
        for neighbor in neighbors {
            if has_cycle(neighbor, graph, visited, stack) {
                return true;
            }
        }
    }

    stack.remove(node);
    false
}

fn count_options(hash_map: &HashMap<String, DynamicType>) -> (usize, usize) {
    let mut count = 0;
    for (_, c_type) in hash_map.clone() {
        if c_type.is_option() {
            count += 1;
        }
    }

    (hash_map.len(), count)
}

fn check_cycle(cycle_types: HashMap<String, Vec<String>>) -> Result<(), String> {
    let mut visited = HashSet::new();
    let mut stack = HashSet::new();

    for type_name in cycle_types.keys() {
        if !visited.contains(type_name)
            && has_cycle(type_name, &cycle_types, &mut visited, &mut stack)
        {
            return Err(format!(
                "Check error: Circular dependency detected in type '{}'. Types cannot reference themselves directly or indirectly.",
                type_name
            ));
        }
    }

    Ok(())
}

fn add_types(state: &mut ProductionSystem, types: Vec<(String, Fields)>) -> Result<(), String> {
    if types.is_empty() {
        return Err("Check error: Cannot add types. The 'types' parameter must contain at least one type definition.".to_owned());
    }

    let mut temporal_types: HashMap<String, Fields> = state.custom_types.clone();

    for (name, fields) in types.clone() {
        temporal_types.insert(name, fields);
    }

    let mut cycle_types: HashMap<String, Vec<String>> = HashMap::new();

    for (name, fields) in temporal_types.clone() {
        if name.is_empty() {
            return Err("Check error: Type name cannot be empty. Please provide a valid type name.".to_owned());
        }
        match name.as_str() {
            "String" | "bool" | "i64" | "f64" | "u64" | "Dummy" | "Option" | "Enum" | "Type"
            | "Vec" => {
                return Err(format!(
                    "Check error: The type name '{}' is reserved and cannot be used. Reserved names are: String, bool, i64, f64, u64, Dummy, Option, Enum, Type, Vec.",
                    name
                ));
            }
            _ => {}
        }

        let internal_types = fields.check_data(temporal_types.clone())?;
        cycle_types.insert(name.clone(), internal_types);
    }

    check_cycle(cycle_types)?;

    state.custom_types = temporal_types;

    Ok(())
}

fn add_unit_process(state: &mut ProductionSystem, add: Vec<UnitProcess>) -> Result<(), String> {
    if add.is_empty() {
        return Err("Check error: Cannot add unit processes. The 'add' parameter must contain at least one unit process definition.".to_owned());
    }

    for unit_process in add {
        unit_process.check_data(&state.custom_types)?;
        state.unit_process.push(unit_process);
    }

    let unit_names = state
        .unit_process
        .iter()
        .map(|x| x.name.clone())
        .collect::<Vec<String>>();
    let hash_unit_name: HashSet<String> = HashSet::from_iter(unit_names.iter().cloned());

    if hash_unit_name.len() != unit_names.len() {
        return Err("Check error: Duplicate unit process names detected. Each unit process must have a unique name.".to_owned());
    }

    Ok(())
}

fn add_new_properties(
    state: &mut ProductionSystem,
    properties: Vec<Properties>,
) -> Result<(), String> {
    if properties.is_empty() {
        return Err("Check error: Cannot add properties. The 'properties' parameter must contain at least one property definition.".to_owned());
    }
    for pro in properties.clone() {
        pro.check_data(&state.custom_types)?;
        state.properties.push(pro);
    }

    let properties_names = state
        .properties
        .iter()
        .map(|x| x.name.clone())
        .collect::<Vec<String>>();
    let hash_properties_names: HashSet<String> =
        HashSet::from_iter(properties_names.iter().cloned());

    if hash_properties_names.len() != properties_names.len() {
        return Err("Check error: Duplicate property names detected. Each property must have a unique name.".to_owned());
    }

    Ok(())
}

fn check_data(
    type_name: &str,
    content: Value,
    custom_types: &HashMap<String, Fields>,
) -> Result<(), String> {
    if type_name.is_empty() {
        return Err("Check error: Type name cannot be empty. Please provide a valid type name for the element.".to_owned());
    }

    if let Some(dynamic_type) = custom_types.get(type_name) {
        dynamic_type.check_value(content, custom_types)
    } else {
        match type_name {
            "String" => DynamicType::String.deserialize(content, custom_types),
            "i64" => DynamicType::i64.deserialize(content, custom_types),
            "u64" => DynamicType::u64.deserialize(content, custom_types),
            "f64" => DynamicType::f64.deserialize(content, custom_types),
            "bool" => DynamicType::bool.deserialize(content, custom_types),
            _ => Err(format!("Check error: Unknown type name '{}'. The type must be either a built-in type (String, i64, u64, f64, bool) or a custom type defined in the schema.", type_name)),
        }
    }
}

fn register_data(
    local_name: &str,
    local_type_name: &str,
    custom_types: &HashMap<String, Fields>,
    name: &str,
    type_name: &str,
    content: Value,
) -> Result<(), String> {
    if local_name != name {
        return Err(format!(
            "Check error: Name mismatch. Expected name '{}' but received '{}'. The data name must match the expected name.",
            local_name, name
        ));
    }

    if local_type_name != type_name {
        return Err(format!(
            "Check error: Type mismatch. Expected type '{}' but received '{}'. The data type must match the expected type.",
            local_type_name, type_name
        ));
    }

    if let Some(c_type) = custom_types.get(local_type_name) {
        c_type.check_value(content, custom_types)?;
    } else {
        match local_type_name {
            "String" => DynamicType::String.deserialize(content, custom_types)?,
            "i64" => DynamicType::i64.deserialize(content, custom_types)?,
            "u64" => DynamicType::u64.deserialize(content, custom_types)?,
            "f64" => DynamicType::f64.deserialize(content, custom_types)?,
            "bool" => DynamicType::bool.deserialize(content, custom_types)?,
            _ => return Err(format!("Check error: Unknown type name '{}'. The type must be either a built-in type (String, i64, u64, f64, bool) or a custom type defined in the schema.", local_type_name)),
        };
    };

    Ok(())
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct ProductionSystem {
    pub name: String,
    pub custom_types: HashMap<String, Fields>,
    pub version: u32,
    pub unit_process: Vec<UnitProcess>,
    pub properties: Vec<Properties>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(untagged)]
pub enum Fields {
    Basic(Box<DynamicType>),
    Object(HashMap<String, DynamicType>),
}

impl Fields {
    fn check_data(&self, custom_types: HashMap<String, Fields>) -> Result<Vec<String>, String> {
        let mut internal_types: Vec<String> = vec![];
        match self {
            Fields::Basic(dynamic_type) => {
                if let DynamicType::Dummy | DynamicType::Option(..) | DynamicType::Type(..) =
                    **dynamic_type
                {
                    return Err("Check error: Invalid basic type. A 'Basic' field cannot be Dummy, Option, or Type. Use 'Object' for complex types or specify a concrete type.".to_owned());
                }

                dynamic_type.check_data(custom_types.clone(), &mut internal_types)?;
            }
            Fields::Object(hash_map) => {
                if hash_map.is_empty() {
                    return Err("Check error: Object fields cannot be empty. An object type must contain at least one field.".to_owned());
                }

                for (field, c_type) in hash_map.iter() {
                    if let DynamicType::Dummy = c_type {
                        return Err(format!("Check error: Field '{}' has invalid type. Object fields cannot be of type 'Dummy'. Please specify a concrete type.", field));
                    }
                    if field.is_empty() {
                        return Err("Check error: Field name cannot be empty. All object fields must have a non-empty name.".to_owned());
                    }

                    c_type.check_data(custom_types.clone(), &mut internal_types)?;
                }
            }
        }

        Ok(internal_types)
    }

    fn check_value(
        &self,
        data: Value,
        custom_types: &HashMap<String, Fields>,
    ) -> Result<(), String> {
        match self {
            Fields::Basic(dynamic_type) => {
                dynamic_type.deserialize(data, custom_types)?;
            }
            Fields::Object(hash_map) => {
                let Some(mut data_object) = data.as_object().cloned() else {
                    return Err("Check error: Type mismatch. Expected an object but received a different type. The data must be a JSON object.".to_owned());
                };

                let (len, options) = count_options(hash_map);
                if data_object.len() < len - options || data_object.len() > len {
                    return Err(format!(
                        "Check error: Field count mismatch. Expected between {} and {} fields but received {}. The data object must match the type definition.",
                        len - options, len, data_object.len()
                    ));
                }

                for (custom_type_name, custom_type_type) in hash_map.clone() {
                    if let Some(field_type) = data_object.remove(&custom_type_name) {
                        custom_type_type.deserialize(field_type, custom_types)?;
                    } else if !custom_type_type.is_option() {
                        return Err(format!(
                            "Check error: Missing required field '{}'. This field is required by the type definition and cannot be omitted.",
                            custom_type_name
                        ));
                    };
                }

                if !data_object.is_empty() {
                    let extra_fields: Vec<String> = data_object.keys().cloned().collect();
                    return Err(format!(
                        "Check error: Unexpected fields found: {:?}. These fields are not defined in the type schema and should be removed.",
                        extra_fields
                    ));
                }
            }
        }

        Ok(())
    }
}

#[allow(non_camel_case_types)]
#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum DynamicType {
    String,
    i64,
    u64,
    f64,
    bool,
    Vec(Box<DynamicType>),
    Enum(HashMap<String, DynamicType>),
    Option(Box<DynamicType>),
    Type(String),
    Dummy,
}

impl DynamicType {
    fn is_option(&self) -> bool {
        matches!(self, DynamicType::Option(_))
    }

    fn deserialize(
        &self,
        value: Value,
        custom_types: &HashMap<String, Fields>,
    ) -> Result<(), String> {
        match self {
            DynamicType::String => {
                if value.as_str().is_none() {
                    return Err(format!("Deserialization error: Expected a String but received '{}'. Please provide a valid string value.", value));
                }
            }
            DynamicType::u64 => {
                if value.as_u64().is_none() {
                    return Err(format!("Deserialization error: Expected an unsigned 64-bit integer (u64) but received '{}'. Please provide a valid non-negative integer.", value));
                }
            }
            DynamicType::f64 => {
                if value.as_f64().is_none() {
                    return Err(format!("Deserialization error: Expected a 64-bit floating point number (f64) but received '{}'. Please provide a valid decimal number.", value));
                }
            }
            DynamicType::i64 => {
                if value.as_i64().is_none() {
                    return Err(format!("Deserialization error: Expected a signed 64-bit integer (i64) but received '{}'. Please provide a valid integer.", value));
                }
            }
            DynamicType::bool => {
                if value.as_bool().is_none() {
                    return Err(format!("Deserialization error: Expected a boolean (true/false) but received '{}'. Please provide a valid boolean value.", value));
                }
            }
            DynamicType::Vec(vec_type) => {
                let Some(vec_dynamic) = value.as_array() else {
                    return Err(format!("Deserialization error: Expected an array but received '{}'. Please provide a valid JSON array.", value));
                };
                for val in vec_dynamic.clone() {
                    vec_type.deserialize(val, custom_types)?;
                }
            }
            DynamicType::Enum(enum_type) => {
                if let Some(obj_dynamic) = value.as_object().cloned() {
                    if obj_dynamic.len() != 1 {
                        return Err(format!(
                            "Deserialization error: Invalid enum object. Expected exactly one field but received {}. Enum values must be represented as a single-field object.",
                            obj_dynamic.len()
                        ));
                    }

                    for (value_name, value_val) in obj_dynamic {
                        let Some(type_dyn) = enum_type.get(&value_name) else {
                            return Err(format!(
                                "Deserialization error: Unknown enum variant '{}'. Valid variants are: {:?}",
                                value_name, enum_type.keys().collect::<Vec<_>>()
                            ));
                        };

                        type_dyn.deserialize(value_val, custom_types)?;
                    }
                } else if let Some(obj_dynamic) = value.as_str() {
                    if let Some(DynamicType::Dummy) = enum_type.get(obj_dynamic) {
                        // Ok
                    } else {
                        return Err(format!(
                            "Deserialization error: Unknown enum variant '{}'. Valid variants are: {:?}",
                            obj_dynamic, enum_type.keys().collect::<Vec<_>>()
                        ));
                    }
                } else {
                    return Err(format!("Deserialization error: Cannot deserialize value '{}' as Enum. Enum values must be either a single-field object or a string (for variants without data).", value));
                };
            }
            DynamicType::Type(c_type) => {
                let Some(obj_type) = custom_types.get(c_type) else {
                    return Err(format!(
                        "Deserialization error: Custom type '{}' is not defined in the schema. Please ensure the type is defined before using it.",
                        c_type
                    ));
                };

                match obj_type {
                    Fields::Basic(type_dyn) => {
                        type_dyn.deserialize(value, custom_types)?;
                    }
                    Fields::Object(hash_map) => {
                        let Some(mut obj_dynamic) = value.as_object().cloned() else {
                            return Err(format!("Deserialization error: Expected an object for custom type '{}' but received '{}'. Please provide a valid JSON object.", c_type, value));
                        };

                        let (len, options) = count_options(hash_map);
                        if obj_dynamic.len() < len - options || obj_dynamic.len() > len {
                            return Err(format!(
                                "Deserialization error: Field count mismatch for custom type '{}'. Expected between {} and {} fields but received {}. Please check the type definition.",
                                c_type, len - options, len, obj_dynamic.len()
                            ));
                        }

                        for (type_field, type_dyn) in hash_map.clone() {
                            if let Some(value) = obj_dynamic.remove(&type_field) {
                                type_dyn.deserialize(value, custom_types)?;
                            } else if !type_dyn.is_option() {
                                return Err(format!(
                                    "Deserialization error: Missing required field '{}' in custom type '{}'. This field is required and cannot be omitted.",
                                    type_field, c_type
                                ));
                            };
                        }

                        if !obj_dynamic.is_empty() {
                            let extra_fields: Vec<String> = obj_dynamic.keys().cloned().collect();
                            return Err(format!(
                                "Deserialization error: Unexpected fields {:?} found in custom type '{}'. These fields are not defined in the type schema.",
                                extra_fields, c_type
                            ));
                        }
                    }
                };
            }
            DynamicType::Option(option) => {
                if value.is_null() {
                    return Ok(());
                } else {
                    return option.deserialize(value, custom_types);
                }
            }
            DynamicType::Dummy => {
                return Err("Check error: Dummy type encountered during deserialization. Dummy types are placeholders and cannot be used for actual data.".to_owned());
            }
        }

        Ok(())
    }

    fn check_data(
        &self,
        custom_types: HashMap<String, Fields>,
        internal_types: &mut Vec<String>,
    ) -> Result<(), String> {
        match self {
            DynamicType::Vec(c_type) | DynamicType::Option(c_type) => {
                if let DynamicType::Dummy | DynamicType::Option(..) = **c_type {
                    return Err("Check error: Invalid nested type. Vec and Option types cannot contain Dummy or nested Option types. Please use a concrete type.".to_owned());
                }
                c_type.check_data(custom_types, internal_types)?;
            }
            DynamicType::Enum(enum_type) => {
                for (type_field, type_dyn) in enum_type.clone() {
                    if type_field.is_empty() {
                        return Err("Check error: Enum variant name cannot be empty. All enum variants must have a non-empty name.".to_owned());
                    }

                    if let DynamicType::Option(..) = type_dyn {
                        return Err(format!("Check error: Enum variant '{}' cannot be of type Option. Use a unit variant (Dummy) for variants without data instead.", type_field));
                    }

                    type_dyn.check_data(custom_types.clone(), internal_types)?;
                }
            }
            DynamicType::Type(c_type) => {
                if c_type.is_empty() {
                    return Err("Check error: Custom type name cannot be empty. Please provide a valid type name.".to_string());
                }

                if !custom_types.contains_key(c_type) {
                    return Err(format!("Check error: Custom type '{}' is not defined. Please ensure the type is defined before referencing it.", c_type));
                }

                internal_types.push(c_type.clone());
            }
            _ => {}
        }

        Ok(())
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct UnitData {
    pub name: String,
    pub outputs: Option<Vec<RegisterData>>,
    pub inputs: Option<Vec<RegisterData>>,
    pub properties: Option<Vec<Properties>>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct UnitProcess {
    pub name: String,
    pub inputs: Vec<Data>,
    pub outputs: Vec<Data>,
    pub properties: Vec<Properties>,
}

impl UnitProcess {
    pub fn check_data(&self, custom_types: &HashMap<String, Fields>) -> Result<(), String> {
        if self.name.is_empty() {
            return Err("Check error: Unit process name cannot be empty. Please provide a valid name for the unit process.".to_owned());
        }

        let mut names = vec![];

        for i in self.inputs.iter() {
            i.check_data(custom_types)?;
            names.push(i.name.clone());
        }

        for o in self.outputs.iter() {
            o.check_data(custom_types)?;
            names.push(o.name.clone());
        }

        let hash_name: HashSet<String> = HashSet::from_iter(names.iter().cloned());

        if hash_name.len() != self.outputs.len() + self.inputs.len() {
            return Err(format!(
                "Check error: Duplicate names detected in unit process '{}'. Input and output names must be unique across both lists.",
                self.name
            ));
        }

        let mut properties_name = vec![];
        for p in self.properties.iter() {
            p.check_data(custom_types)?;
            properties_name.push(p.name.clone());
        }

        let hash_pro_name: HashSet<String> = HashSet::from_iter(properties_name.iter().cloned());
        if hash_name.len() != self.outputs.len() + self.inputs.len() {
            return Err(format!(
                "Check error: Duplicate names detected in unit process '{}'. Input and output names must be unique across both lists.",
                self.name
            ));
        }

        if hash_pro_name.len() != self.properties.len() {
            return Err(format!(
                "Check error: Duplicate property names detected in unit process '{}'. Each property must have a unique name.",
                self.name
            ));
        }

        Ok(())
    }

    pub fn register_data(
        &mut self,
        unit: UnitData,
        custom_types: &HashMap<String, Fields>,
    ) -> Result<(), String> {
        if unit.inputs.is_none() && unit.outputs.is_none() {
            return Err(format!(
                "Check error: Cannot register data for unit '{}'. At least one of 'inputs' or 'outputs' must be provided.",
                unit.name
            ));
        }

        if let Some(inputs) = unit.inputs {
            if inputs.is_empty() {
                return Err(format!(
                    "Check error: Empty inputs array for unit '{}'. If 'inputs' is provided, it must contain at least one input definition.",
                    unit.name
                ));
            }

            let mut updates: usize = 0;

            for element_state in self.inputs.iter_mut() {
                for element_unit in inputs.clone() {
                    if element_state.name == element_unit.name {
                        let element_unit = Data::from(element_unit);
                        element_state.register_data(element_unit, custom_types)?;
                        updates += 1;
                    }
                }
            }

            if updates != inputs.len() {
                let unmatched = inputs.len() - updates;
                return Err(format!(
                    "Check error: Failed to update {} input(s) in unit process '{}'. {} input name(s) do not match any defined inputs in the unit process.",
                    unmatched, self.name, unmatched
                ));
            }
        }

        if let Some(outputs) = unit.outputs {
            if outputs.is_empty() {
                return Err(format!(
                    "Check error: Empty outputs array for unit '{}'. If 'outputs' is provided, it must contain at least one output definition.",
                    unit.name
                ));
            }

            let mut updates: usize = 0;

            for element_state in self.outputs.iter_mut() {
                for element_unit in outputs.clone() {
                    if element_state.name == element_unit.name {
                        let element_unit = Data::from(element_unit);
                        element_state.register_data(element_unit, custom_types)?;
                        updates += 1;
                    }
                }
            }

            if updates != outputs.len() {
                let unmatched = outputs.len() - updates;
                return Err(format!(
                    "Check error: Failed to update {} output(s) in unit process '{}'. {} output name(s) do not match any defined outputs in the unit process.",
                    unmatched, self.name, unmatched
                ));
            }
        }

        if let Some(properties) = unit.properties {
            if properties.is_empty() {
                return Err(format!(
                    "Check error: Empty properties array for unit '{}'. If 'properties' is provided, it must contain at least one property definition.",
                    unit.name
                ));
            }

            let mut updates: usize = 0;

            for data in self.properties.iter_mut() {
                for unit_data in properties.clone() {
                    if data.name == unit_data.name {
                        data.register_data(unit_data, custom_types)?;
                        updates += 1;
                    }
                }
            }

            if updates != properties.len() {
                let unmatched = properties.len() - updates;
                return Err(format!(
                    "Check error: Failed to update {} propert(y/ies) in unit process '{}'. {} property name(s) do not match any defined properties in the unit process.",
                    unmatched, self.name, unmatched
                ));
            }
        }

        Ok(())
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct Target {
    pub governance_id: String,
    pub subject_id: String,
    pub unit_process: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct Properties {
    pub name: String,
    pub type_name: String,
    pub content: Value,
}

impl Properties {
    fn check_data(&self, custom_types: &HashMap<String, Fields>) -> Result<(), String> {
        if self.name.is_empty() {
            return Err("Check error: Property name cannot be empty. Please provide a valid name for the property.".to_owned());
        }

        check_data(&self.type_name, self.content.clone(), custom_types)
    }

    fn register_data(
        &mut self,
        data: Self,
        custom_types: &HashMap<String, Fields>,
    ) -> Result<(), String> {
        register_data(
            &self.name,
            &self.type_name,
            custom_types,
            &data.name,
            &data.type_name,
            data.content.clone(),
        )?;

        self.content = data.content;

        Ok(())
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct Data {
    pub name: String,
    pub type_name: String,
    pub metadata: Option<Metadata>,
    pub content: Value,
    pub targets: Option<Vec<Target>>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct Metadata {
    pub type_name: String,
    pub content: Value,
}

impl From<RegisterData> for Data {
    fn from(value: RegisterData) -> Self {
        Data {
            name: value.name,
            type_name: value.type_name,
            metadata: None,
            content: value.content,
            targets: value.targets,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct RegisterData {
    name: String,
    type_name: String,
    content: Value,
    targets: Option<Vec<Target>>,
}

impl Data {
    fn check_data(&self, custom_types: &HashMap<String, Fields>) -> Result<(), String> {
        if self.targets.is_some() {
            return Err(format!(
                "Check error: Targets must not be set in unit process definition for element '{}'. Targets are only valid when registering data.",
                self.name
            ));
        }

        if self.name.is_empty() {
            return Err("Check error: Data element name cannot be empty. Please provide a valid name for the data element.".to_owned());
        }

        if let Some(metadata) = self.metadata.clone() {
            check_data(&metadata.type_name, metadata.content.clone(), custom_types)?;
        };

        check_data(&self.type_name, self.content.clone(), custom_types)
    }

    fn register_data(
        &mut self,
        data: Self,
        custom_types: &HashMap<String, Fields>,
    ) -> Result<(), String> {
        register_data(
            &self.name,
            &self.type_name,
            custom_types,
            &data.name,
            &data.type_name,
            data.content.clone(),
        )?;

        if let Some(targets) = self.targets.clone() {
            for t in targets {
                if t.governance_id.is_empty()
                    || t.subject_id.is_empty()
                    || t.unit_process.is_empty()
                {
                    return Err(format!(
                        "Check error: Invalid target configuration for data element '{}'. All target fields (governance_id, subject_id, unit_process) must be non-empty.",
                        self.name
                    ));
                }
            }
        }

        self.content = data.content;
        self.targets = data.targets;

        Ok(())
    }
}

#[derive(Serialize, Deserialize, Clone)]
enum Events {
    ChangeProductionSystem(ChangeProductionSystem),
    RegisterData(Vec<UnitData>),
}

#[derive(Serialize, Deserialize, Clone)]
enum ChangeProductionSystem {
    Init {
        name: String,
        unit_process: Option<Vec<UnitProcess>>,
        types: Option<Vec<(String, Fields)>>,
        properties: Option<Vec<Properties>>,
    },
    ModifyProductionSystem {
        name: Option<String>,
        delete_properties: Option<Vec<String>>,
        modify_properties: Option<Vec<(String, Properties)>>,
        add_properties: Option<Vec<Properties>>,
    },
    ModifyTypes {
        delete: Option<Vec<String>>,
        add: Option<Vec<(String, Fields)>>,
    },
    ModifyUnitProcess {
        delete: Option<Vec<String>>,
        modify: Option<Vec<(String, UnitProcess)>>,
        add: Option<Vec<UnitProcess>>,
    },
}

#[unsafe(no_mangle)]
pub unsafe fn main_function(
    state_ptr: i32,
    init_state_ptr: i32,
    event_ptr: i32,
    is_owner: i32,
) -> u32 {
    sdk::execute_contract(
        state_ptr,
        init_state_ptr,
        event_ptr,
        is_owner,
        contract_logic,
    )
}

#[unsafe(no_mangle)]
pub unsafe fn init_check_function(state_ptr: i32) -> u32 {
    sdk::check_init_data(state_ptr, init_logic)
}

fn init_logic(_state: &ProductionSystem, contract_result: &mut sdk::ContractInitCheck) {
    contract_result.success = true;
}

fn contract_logic(
    context: &sdk::Context<Events>,
    contract_result: &mut sdk::ContractResult<ProductionSystem>,
) {
    let state = &mut contract_result.state;

    if let Events::ChangeProductionSystem(ChangeProductionSystem::Init { .. }) = context.event {
        if state.version != 0 {
            contract_result.error = "Contract error: Cannot emit Init event when version is not 0. The Init event can only be used to initialize a new contract (version must be 0).".to_owned();
            return;
        }
    } else if state.version == 0 {
        contract_result.error = "Contract error: The first event must be an Init event. Please initialize the contract with an Init event before performing other operations.".to_owned();
        return;
    }

    if let Events::ChangeProductionSystem(..) = context.event {
        state.version += 1;
    }

    match context.event.clone() {
        Events::ChangeProductionSystem(operation) => match operation {
            ChangeProductionSystem::Init {
                name,
                unit_process,
                properties,
                types,
            } => {
                if name.is_empty() {
                    contract_result.error = "Init error: Production system name cannot be empty. Please provide a valid name for the production system.".to_owned();
                    return;
                }

                state.name = name;

                if let Some(types) = types {
                    if let Err(e) = add_types(state, types) {
                        contract_result.error = e;
                        return;
                    };
                }

                if let Some(unit_process) = unit_process {
                    if let Err(e) = add_unit_process(state, unit_process) {
                        contract_result.error = e;
                        return;
                    }
                }

                if let Some(properties) = properties {
                    if let Err(e) = add_new_properties(state, properties) {
                        contract_result.error = e;
                        return;
                    }
                }
            }
            ChangeProductionSystem::ModifyProductionSystem {
                name,
                delete_properties,
                add_properties,
                modify_properties,
            } => {
                if name.is_none()
                    && delete_properties.is_none()
                    && add_properties.is_none()
                    && modify_properties.is_none()
                {
                    contract_result.error = "ModifyProductionSystem error: At least one parameter must be provided. Please specify 'name', 'delete_properties', 'add_properties', or 'modify_properties'.".to_owned();
                    return;
                }

                if let Some(name) = name {
                    if name.is_empty() {
                        contract_result.error = "ModifyProductionSystem error: New production system name cannot be empty. Please provide a valid name.".to_owned();
                        return;
                    }

                    state.name = name;
                }

                if let Some(delete_properties) = delete_properties {
                    if delete_properties.is_empty() {
                        contract_result.error = "ModifyProductionSystem error: The 'delete_properties' list cannot be empty. Please specify at least one property to delete.".to_owned();
                        return;
                    }

                    for name in delete_properties.iter() {
                        if let Some(pos) = state.properties.iter().position(|x| x.name == *name) {
                            state.properties.remove(pos);
                        } else {
                            contract_result.error = format!(
                                "ModifyProductionSystem error: Cannot delete property '{}'. This property does not exist in the production system.",
                                name
                            );
                            return;
                        }
                    }
                }

                if let Some(modify_properties) = modify_properties {
                    if modify_properties.is_empty() {
                        contract_result.error = "ModifyProductionSystem error: The 'modify_properties' list cannot be empty. Please specify at least one property to modify.".to_owned();
                        return;
                    }

                    for (name, propiertie) in modify_properties.clone() {
                        if let Err(e) = propiertie.check_data(&state.custom_types) {
                            contract_result.error = e;
                            return;
                        };

                        if let Some(existing) =
                            state.properties.iter_mut().find(|x| x.name == name)
                        {
                            *existing = propiertie;
                        } else {
                            contract_result.error = format!(
                                "ModifyProductionSystem error: Cannot modify property '{}'. This property does not exist in the production system.",
                                name
                            );
                            return;
                        }
                    }
                }

                if let Some(add_properties) = add_properties {
                    if let Err(e) = add_new_properties(state, add_properties) {
                        contract_result.error = e;
                        return;
                    }
                }
            }
            ChangeProductionSystem::ModifyTypes { delete, add } => {
                if delete.is_none() && add.is_none() {
                    contract_result.error = "ModifyTypes error: At least one parameter must be provided. Please specify 'add' or 'delete'.".to_owned();
                    return;
                }

                if let Some(delete) = delete {
                    for name in delete {
                        if state.custom_types.remove(&name).is_none() {
                            contract_result.error = format!(
                                "ModifyTypes error: Cannot delete type '{}'. This type does not exist in the schema.",
                                name
                            );
                            return;
                        }
                    }
                }

                if let Some(add) = add {
                    if let Err(e) = add_types(state, add) {
                        contract_result.error = e;
                        return;
                    };
                }
            }
            ChangeProductionSystem::ModifyUnitProcess {
                modify,
                add,
                delete,
            } => {
                if delete.is_none() && add.is_none() && modify.is_none() {
                    contract_result.error = "ModifyUnitProcess error: At least one parameter must be provided. Please specify 'add', 'modify', or 'delete'.".to_owned();
                    return;
                }

                if let Some(delete) = delete {
                    if delete.is_empty() {
                        contract_result.error = "ModifyUnitProcess error: The 'delete' list cannot be empty. Please specify at least one unit process to delete.".to_owned();
                        return;
                    }

                    for name in delete.clone() {
                        if let Some(pos) = state.unit_process.iter().position(|x| x.name == name) {
                            state.unit_process.remove(pos);
                        } else {
                            contract_result.error = format!(
                                "ModifyUnitProcess error: Cannot delete unit process '{}'. This unit process does not exist in the production system.",
                                name
                            );
                            return;
                        }
                    }
                }

                if let Some(modify) = modify {
                    if modify.is_empty() {
                        contract_result.error = "ModifyUnitProcess error: The 'modify' list cannot be empty. Please specify at least one unit process to modify.".to_owned();
                        return;
                    }

                    for (name, process) in modify.clone() {
                        if let Err(e) = process.check_data(&state.custom_types) {
                            contract_result.error = e;
                            return;
                        };

                        if let Some(existing) =
                            state.unit_process.iter_mut().find(|x| x.name == name)
                        {
                            *existing = process;
                        } else {
                            contract_result.error = format!(
                                "ModifyUnitProcess error: Cannot modify unit process '{}'. This unit process does not exist in the production system.",
                                name
                            );
                            return;
                        }
                    }
                }

                if let Some(add) = add {
                    if let Err(e) = add_unit_process(state, add) {
                        contract_result.error = e;
                        return;
                    }
                }
            }
        },
        Events::RegisterData(data) => {
            if state.version == 0 {
                contract_result.error = "RegisterData error: Cannot register data before initialization. The first event must be an Init event.".to_owned();
                return;
            }

            if data.is_empty() {
                contract_result.error = "RegisterData error: The data list cannot be empty. Please provide at least one unit data entry to register.".to_owned();
                return;
            }

            for d in data {
                let mut change = false;

                for unit_process in state.unit_process.iter_mut() {
                    if unit_process.name == d.name {
                        if let Err(e) = unit_process.register_data(d.clone(), &state.custom_types) {
                            contract_result.error = e;
                            return;
                        };
                        change = true;
                        break;
                    }
                }

                if !change {
                    contract_result.error = format!(
                        "RegisterData error: No unit process found with name '{}'. Please ensure the unit process exists before registering data to it.",
                        d.name
                    );
                    return;
                }
            }
        }
    }

    contract_result.success = true;
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, vec};

    use crate::{
        contract_logic, ChangeProductionSystem, Data, DynamicType, Events, Fields, Metadata, ProductionSystem, Properties, RegisterData, Target, UnitData, UnitProcess
    };
    use kore_contract_sdk as sdk;
    use serde_json::json;

    impl PartialEq for DynamicType {
        fn eq(&self, other: &Self) -> bool {
            use DynamicType::*;
            match (self, other) {
                (String, String) => true,
                (i64, i64) => true,
                (u64, u64) => true,
                (f64, f64) => true,
                (bool, bool) => true,
                (Vec(a), Vec(b)) => a == b,
                (Option(a), Option(b)) => a == b,
                (Enum(a), Enum(b)) => a == b,
                (Type(a), Type(b)) => a == b,
                (Dummy, Dummy) => true,
                _ => false,
            }
        }
    }

    impl Eq for DynamicType {}

    impl PartialEq for Fields {
        fn eq(&self, other: &Self) -> bool {
            use Fields::*;
            match (self, other) {
                (Basic(a), Basic(b)) => a == b,
                (Object(a), Object(b)) => a == b,
                _ => false,
            }
        }
    }

    impl Eq for Fields {}

    impl PartialEq for Data {
        fn eq(&self, other: &Self) -> bool {
            self.name == other.name
                && self.type_name == other.type_name
                && self.content == other.content
                && self.targets == other.targets
        }
    }

    impl Eq for Data {}

    impl PartialEq for Metadata {
        fn eq(&self, other: &Self) -> bool {
            self.type_name == other.type_name
                && self.content == other.content
        }
    }

    impl Eq for Metadata {}


    impl PartialEq for Target {
        fn eq(&self, other: &Self) -> bool {
            self.governance_id == other.governance_id
                && self.subject_id == other.subject_id
                && self.unit_process == other.unit_process
        }
    }

    impl Eq for Target {}

    

    impl PartialEq for Properties {
        fn eq(&self, other: &Self) -> bool {
            self.name == other.name
                && self.type_name == other.type_name
                && self.content == other.content
        }
    }
    
    impl Eq for Properties {}

    impl PartialEq for UnitProcess {
        fn eq(&self, other: &Self) -> bool {
            self.name == other.name
                && self.inputs == other.inputs
                && self.outputs == other.outputs
                && self.properties == other.properties
        }
    }

    impl Eq for UnitProcess {}

    #[test]
    fn register_types_type_cycle() {
        let mut custom_type = HashMap::new();
        custom_type.insert("name".to_owned(), DynamicType::String);
        custom_type.insert(
            "value".to_owned(),
            DynamicType::Type("Another User".to_owned()),
        );

        let custom_type = Fields::Object(custom_type);

        let mut types = HashMap::new();
        types.insert("User".to_owned(), custom_type);

        let init_state = ProductionSystem {
            name: "wine process".to_owned(),
            version: 1,
            unit_process: vec![],
            custom_types: types,
            properties: vec![],
        };

        let mut custom_type = HashMap::new();
        custom_type.insert(
            "user_data".to_owned(),
            DynamicType::Type("Fake User".to_owned()),
        );
        custom_type.insert("value".to_owned(), DynamicType::String);
        let custom_type = Fields::Object(custom_type);

        let mut custom_type_2 = HashMap::new();
        custom_type_2.insert("user_data".to_owned(), DynamicType::Type("User".to_owned()));
        custom_type_2.insert("value".to_owned(), DynamicType::String);

        let custom_type_2 = Fields::Object(custom_type_2);

        let context = sdk::Context {
            event: Events::ChangeProductionSystem(ChangeProductionSystem::ModifyTypes {
                delete: None,
                add: Some(vec![
                    ("Another User".to_owned(), custom_type),
                    ("Fake User".to_owned(), custom_type_2),
                ]),
            }),
            is_owner: false,
        };

        let mut result = sdk::ContractResult::new(init_state.clone());

        contract_logic(&context, &mut result);

        assert!(!result.error.is_empty());
        assert!(!result.success);
    }

        #[test]
    fn test_metadata_field() {
        ////////////////////////////////////////////////////////////////
        // Type definition
        ////////////////////////////////////////////////////////////////
        let init_state = ProductionSystem {
            name: "example".to_owned(),
            version: 1,
            unit_process: vec![],
            custom_types: HashMap::new(),
            properties: vec![],
        };

        let context = sdk::Context {
            event: Events::ChangeProductionSystem(ChangeProductionSystem::ModifyTypes {
                delete: None,
                add: Some(vec![
                    (
                        "UserObject".to_owned(),
                        Fields::Object(HashMap::from([("name".to_owned(), DynamicType::String)])),
                    ),
                    (
                        "UserBasic".to_owned(),
                        Fields::Basic(Box::new(DynamicType::String)),
                    ),
                ]),
            }),
            is_owner: false,
        };

        let mut result = sdk::ContractResult::new(init_state.clone());

        contract_logic(&context, &mut result);

        assert!(result.error.is_empty());
        assert_eq!(
            result.state.custom_types.get("UserObject").unwrap().clone(),
            Fields::Object(HashMap::from([("name".to_owned(), DynamicType::String)]))
        );
        assert_eq!(
            result.state.custom_types.get("UserBasic").unwrap().clone(),
            Fields::Basic(Box::new(DynamicType::String))
        );
        assert_eq!(result.state.custom_types.len(), 2);

        ////////////////////////////////////////////////////////////////
        // Unit process
        ////////////////////////////////////////////////////////////////
        let unit_process = UnitProcess {
            name: "Unit example".to_owned(),
            outputs: vec![
                Data {
                    name: "Example Object".to_owned(),
                    type_name: "UserObject".to_owned(),
                    content: json!({"name": "ExampleName"}),
                    targets: None,
                    metadata: None,
                },
                Data {
                    name: "Example String".to_owned(),
                    type_name: "String".to_owned(),
                    content: json!("ExampleString"),
                    targets: None,
                    metadata: None,
                },
            ],
            inputs: vec![Data {
                name: "Example Basic".to_owned(),
                type_name: "UserBasic".to_owned(),
                content: json!("ExampleBasic"),
                targets: None,
                metadata: Some(Metadata { type_name: "UserObject".to_owned(), content: json!({"name": "Metadata"}) }),
            }],
            properties: vec![],
        };

        let context = sdk::Context {
            event: Events::ChangeProductionSystem(ChangeProductionSystem::ModifyUnitProcess {
                delete: None,
                modify: None,
                add: Some(vec![unit_process]),
            }),
            is_owner: false,
        };

        let mut result = sdk::ContractResult::new(result.state);
        contract_logic(&context, &mut result);

        println!("{}", result.error);
        assert!(result.error.is_empty());

        assert_eq!(result.state.unit_process.len(), 1);
        assert_eq!(result.state.unit_process[0].name, "Unit example");
        assert_eq!(result.state.unit_process[0].outputs.len(), 2);

        let data = result.state.unit_process[0].outputs[0].clone();
        assert_eq!(data.name, "Example Object");
        assert_eq!(data.type_name, "UserObject");
        assert_eq!(data.content, json!({"name": "ExampleName"}));
        let data = result.state.unit_process[0].outputs[1].clone();
        assert_eq!(data.name, "Example String");
        assert_eq!(data.type_name, "String");
        assert_eq!(data.content, json!("ExampleString"));

        let data = result.state.unit_process[0].inputs[0].clone();
        assert_eq!(data.name, "Example Basic");
        assert_eq!(data.type_name, "UserBasic");
        assert_eq!(data.content, json!("ExampleBasic"));
        assert_eq!(data.metadata.unwrap(), Metadata { type_name: "UserObject".to_owned(), content: json!({"name": "Metadata"}) });

        assert!(result.success);
    }

    #[test]
    fn test_string() {
        ////////////////////////////////////////////////////////////////
        // Type definition
        ////////////////////////////////////////////////////////////////
        let init_state = ProductionSystem {
            name: "example".to_owned(),
            version: 1,
            unit_process: vec![],
            custom_types: HashMap::new(),
            properties: vec![],
        };

        let context = sdk::Context {
            event: Events::ChangeProductionSystem(ChangeProductionSystem::ModifyTypes {
                delete: None,
                add: Some(vec![
                    (
                        "UserObject".to_owned(),
                        Fields::Object(HashMap::from([("name".to_owned(), DynamicType::String)])),
                    ),
                    (
                        "UserBasic".to_owned(),
                        Fields::Basic(Box::new(DynamicType::String)),
                    ),
                ]),
            }),
            is_owner: false,
        };

        let mut result = sdk::ContractResult::new(init_state.clone());

        contract_logic(&context, &mut result);

        assert!(result.error.is_empty());
        assert_eq!(
            result.state.custom_types.get("UserObject").unwrap().clone(),
            Fields::Object(HashMap::from([("name".to_owned(), DynamicType::String)]))
        );
        assert_eq!(
            result.state.custom_types.get("UserBasic").unwrap().clone(),
            Fields::Basic(Box::new(DynamicType::String))
        );
        assert_eq!(result.state.custom_types.len(), 2);

        ////////////////////////////////////////////////////////////////
        // Unit process
        ////////////////////////////////////////////////////////////////
        let unit_process = UnitProcess {
            name: "Unit example".to_owned(),
            outputs: vec![
                Data {
                    name: "Example Object".to_owned(),
                    type_name: "UserObject".to_owned(),
                    content: json!({"name": "ExampleName"}),
                    targets: None,
                    metadata: None,
                },
                Data {
                    name: "Example String".to_owned(),
                    type_name: "String".to_owned(),
                    content: json!("ExampleString"),
                    targets: None,
                    metadata: None,
                },
            ],
            inputs: vec![Data {
                name: "Example Basic".to_owned(),
                type_name: "UserBasic".to_owned(),
                content: json!("ExampleBasic"),
                targets: None,
                metadata: None,
            }],
            properties: vec![],
        };

        let context = sdk::Context {
            event: Events::ChangeProductionSystem(ChangeProductionSystem::ModifyUnitProcess {
                delete: None,
                modify: None,
                add: Some(vec![unit_process]),
            }),
            is_owner: false,
        };

        let mut result = sdk::ContractResult::new(result.state);
        contract_logic(&context, &mut result);

        println!("{}", result.error);
        assert!(result.error.is_empty());

        assert_eq!(result.state.unit_process.len(), 1);
        assert_eq!(result.state.unit_process[0].name, "Unit example");
        assert_eq!(result.state.unit_process[0].outputs.len(), 2);

        let data = result.state.unit_process[0].outputs[0].clone();
        assert_eq!(data.name, "Example Object");
        assert_eq!(data.type_name, "UserObject");
        assert_eq!(data.content, json!({"name": "ExampleName"}));
        let data = result.state.unit_process[0].outputs[1].clone();
        assert_eq!(data.name, "Example String");
        assert_eq!(data.type_name, "String");
        assert_eq!(data.content, json!("ExampleString"));

        let data = result.state.unit_process[0].inputs[0].clone();
        assert_eq!(data.name, "Example Basic");
        assert_eq!(data.type_name, "UserBasic");
        assert_eq!(data.content, json!("ExampleBasic"));

        assert!(result.success);
    }

    #[test]
    fn test_i64() {
        ////////////////////////////////////////////////////////////////
        // Type definition
        ////////////////////////////////////////////////////////////////
        let init_state = ProductionSystem {
            name: "example".to_owned(),
            version: 1,
            unit_process: vec![],
            custom_types: HashMap::new(),
            properties: vec![],
        };

        let context = sdk::Context {
            event: Events::ChangeProductionSystem(ChangeProductionSystem::ModifyTypes {
                delete: None,
                add: Some(vec![
                    (
                        "UserObject".to_owned(),
                        Fields::Object(HashMap::from([("value".to_owned(), DynamicType::i64)])),
                    ),
                    (
                        "UserBasic".to_owned(),
                        Fields::Basic(Box::new(DynamicType::i64)),
                    ),
                ]),
            }),
            is_owner: false,
        };

        let mut result = sdk::ContractResult::new(init_state.clone());

        contract_logic(&context, &mut result);

        assert!(result.error.is_empty());
        assert_eq!(
            result.state.custom_types.get("UserObject").unwrap().clone(),
            Fields::Object(HashMap::from([("value".to_owned(), DynamicType::i64)]))
        );
        assert_eq!(
            result.state.custom_types.get("UserBasic").unwrap().clone(),
            Fields::Basic(Box::new(DynamicType::i64))
        );
        assert_eq!(result.state.custom_types.len(), 2);

        ////////////////////////////////////////////////////////////////
        // Unit process
        ////////////////////////////////////////////////////////////////
        let unit_process = UnitProcess {
            name: "Unit example".to_owned(),
            outputs: vec![
                Data {
                    name: "Example Object".to_owned(),
                    type_name: "UserObject".to_owned(),
                    content: json!({"value": -5}),
                    targets: None,
                    metadata: None,
                },
                Data {
                    name: "Example i64".to_owned(),
                    type_name: "i64".to_owned(),
                    content: json!(21412),
                    targets: None,
                    metadata: None,
                },
            ],
            inputs: vec![Data {
                name: "Example Basic".to_owned(),
                type_name: "UserBasic".to_owned(),
                content: json!(-132),
                targets: None,
                metadata: None,
            }],
            properties: vec![],
        };

        let context = sdk::Context {
            event: Events::ChangeProductionSystem(ChangeProductionSystem::ModifyUnitProcess {
                delete: None,
                modify: None,
                add: Some(vec![unit_process]),
            }),
            is_owner: false,
        };

        let mut result = sdk::ContractResult::new(result.state);
        contract_logic(&context, &mut result);

        println!("{}", result.error);
        assert!(result.error.is_empty());

        assert_eq!(result.state.unit_process.len(), 1);
        assert_eq!(result.state.unit_process[0].name, "Unit example");
        assert_eq!(result.state.unit_process[0].outputs.len(), 2);

        let data = result.state.unit_process[0].outputs[0].clone();
        assert_eq!(data.name, "Example Object");
        assert_eq!(data.type_name, "UserObject");
        assert_eq!(data.content, json!({"value": -5}));
        let data = result.state.unit_process[0].outputs[1].clone();
        assert_eq!(data.name, "Example i64");
        assert_eq!(data.type_name, "i64");
        assert_eq!(data.content, json!(21412),);

        let data = result.state.unit_process[0].inputs[0].clone();
        assert_eq!(data.name, "Example Basic");
        assert_eq!(data.type_name, "UserBasic");
        assert_eq!(data.content, json!(-132),);

        assert!(result.success);
    }

    #[test]
    fn test_u64() {
        ////////////////////////////////////////////////////////////////
        // Type definition
        ////////////////////////////////////////////////////////////////
        let init_state = ProductionSystem {
            name: "example".to_owned(),
            version: 1,
            unit_process: vec![],
            custom_types: HashMap::new(),
            properties: vec![],
        };

        let context = sdk::Context {
            event: Events::ChangeProductionSystem(ChangeProductionSystem::ModifyTypes {
                delete: None,
                add: Some(vec![
                    (
                        "UserObject".to_owned(),
                        Fields::Object(HashMap::from([("value".to_owned(), DynamicType::u64)])),
                    ),
                    (
                        "UserBasic".to_owned(),
                        Fields::Basic(Box::new(DynamicType::u64)),
                    ),
                ]),
            }),
            is_owner: false,
        };

        let mut result = sdk::ContractResult::new(init_state.clone());

        contract_logic(&context, &mut result);

        assert!(result.error.is_empty());
        assert_eq!(
            result.state.custom_types.get("UserObject").unwrap().clone(),
            Fields::Object(HashMap::from([("value".to_owned(), DynamicType::u64)]))
        );
        assert_eq!(
            result.state.custom_types.get("UserBasic").unwrap().clone(),
            Fields::Basic(Box::new(DynamicType::u64))
        );
        assert_eq!(result.state.custom_types.len(), 2);

        ////////////////////////////////////////////////////////////////
        // Unit process
        ////////////////////////////////////////////////////////////////
        let unit_process = UnitProcess {
            name: "Unit example".to_owned(),
            outputs: vec![
                Data {
                    name: "Example Object".to_owned(),
                    type_name: "UserObject".to_owned(),
                    content: json!({"value": 0}),
                    targets: None,
                    metadata: None,
                },
                Data {
                    name: "Example u64".to_owned(),
                    type_name: "u64".to_owned(),
                    content: json!(21412),
                    targets: None,
                    metadata: None,
                },
            ],
            inputs: vec![Data {
                name: "Example Basic".to_owned(),
                type_name: "UserBasic".to_owned(),
                content: json!(132),
                targets: None,
                metadata: None,
            }],
            properties: vec![],
        };

        let context = sdk::Context {
            event: Events::ChangeProductionSystem(ChangeProductionSystem::ModifyUnitProcess {
                delete: None,
                modify: None,
                add: Some(vec![unit_process]),
            }),
            is_owner: false,
        };

        let mut result = sdk::ContractResult::new(result.state);
        contract_logic(&context, &mut result);

        println!("{}", result.error);
        assert!(result.error.is_empty());

        assert_eq!(result.state.unit_process.len(), 1);
        assert_eq!(result.state.unit_process[0].name, "Unit example");
        assert_eq!(result.state.unit_process[0].outputs.len(), 2);

        let data = result.state.unit_process[0].outputs[0].clone();
        assert_eq!(data.name, "Example Object");
        assert_eq!(data.type_name, "UserObject");
        assert_eq!(data.content, json!({"value": 0}));
        let data = result.state.unit_process[0].outputs[1].clone();
        assert_eq!(data.name, "Example u64");
        assert_eq!(data.type_name, "u64");
        assert_eq!(data.content, json!(21412),);

        let data = result.state.unit_process[0].inputs[0].clone();
        assert_eq!(data.name, "Example Basic");
        assert_eq!(data.type_name, "UserBasic");
        assert_eq!(data.content, json!(132),);

        assert!(result.success);
    }

    #[test]
    fn test_f64() {
        ////////////////////////////////////////////////////////////////
        // Type definition
        ////////////////////////////////////////////////////////////////
        let init_state = ProductionSystem {
            name: "example".to_owned(),
            version: 1,
            unit_process: vec![],
            custom_types: HashMap::new(),
            properties: vec![],
        };

        let context = sdk::Context {
            event: Events::ChangeProductionSystem(ChangeProductionSystem::ModifyTypes {
                delete: None,
                add: Some(vec![
                    (
                        "UserObject".to_owned(),
                        Fields::Object(HashMap::from([("value".to_owned(), DynamicType::f64)])),
                    ),
                    (
                        "UserBasic".to_owned(),
                        Fields::Basic(Box::new(DynamicType::f64)),
                    ),
                ]),
            }),
            is_owner: false,
        };

        let mut result = sdk::ContractResult::new(init_state.clone());

        contract_logic(&context, &mut result);

        assert!(result.error.is_empty());
        assert_eq!(
            result.state.custom_types.get("UserObject").unwrap().clone(),
            Fields::Object(HashMap::from([("value".to_owned(), DynamicType::f64)]))
        );
        assert_eq!(
            result.state.custom_types.get("UserBasic").unwrap().clone(),
            Fields::Basic(Box::new(DynamicType::f64))
        );
        assert_eq!(result.state.custom_types.len(), 2);

        ////////////////////////////////////////////////////////////////
        // Unit process
        ////////////////////////////////////////////////////////////////
        let unit_process = UnitProcess {
            name: "Unit example".to_owned(),
            outputs: vec![
                Data {
                    name: "Example Object".to_owned(),
                    type_name: "UserObject".to_owned(),
                    content: json!({"value": 0}),
                    targets: None,
                    metadata: None,
                },
                Data {
                    name: "Example f64".to_owned(),
                    type_name: "f64".to_owned(),
                    content: json!(21412.0),
                    targets: None,
                    metadata: None,
                },
            ],
            inputs: vec![Data {
                name: "Example Basic".to_owned(),
                type_name: "UserBasic".to_owned(),
                content: json!(-132.55),
                targets: None,
                metadata: None,
            }],
            properties: vec![],
        };

        let context = sdk::Context {
            event: Events::ChangeProductionSystem(ChangeProductionSystem::ModifyUnitProcess {
                delete: None,
                modify: None,
                add: Some(vec![unit_process]),
            }),
            is_owner: false,
        };

        let mut result = sdk::ContractResult::new(result.state);
        contract_logic(&context, &mut result);

        println!("{}", result.error);
        assert!(result.error.is_empty());

        assert_eq!(result.state.unit_process.len(), 1);
        assert_eq!(result.state.unit_process[0].name, "Unit example");
        assert_eq!(result.state.unit_process[0].outputs.len(), 2);

        let data = result.state.unit_process[0].outputs[0].clone();
        assert_eq!(data.name, "Example Object");
        assert_eq!(data.type_name, "UserObject");
        assert_eq!(data.content, json!({"value": 0}));
        let data = result.state.unit_process[0].outputs[1].clone();
        assert_eq!(data.name, "Example f64");
        assert_eq!(data.type_name, "f64");
        assert_eq!(data.content, json!(21412.0),);

        let data = result.state.unit_process[0].inputs[0].clone();
        assert_eq!(data.name, "Example Basic");
        assert_eq!(data.type_name, "UserBasic");
        assert_eq!(data.content, json!(-132.55),);

        assert!(result.success);
    }

    #[test]
    fn test_bool() {
        ////////////////////////////////////////////////////////////////
        // Type definition
        ////////////////////////////////////////////////////////////////
        let init_state = ProductionSystem {
            name: "example".to_owned(),
            version: 1,
            unit_process: vec![],
            custom_types: HashMap::new(),
            properties: vec![],
        };

        let context = sdk::Context {
            event: Events::ChangeProductionSystem(ChangeProductionSystem::ModifyTypes {
                delete: None,
                add: Some(vec![
                    (
                        "UserObject".to_owned(),
                        Fields::Object(HashMap::from([("value".to_owned(), DynamicType::bool)])),
                    ),
                    (
                        "UserBasic".to_owned(),
                        Fields::Basic(Box::new(DynamicType::bool)),
                    ),
                ]),
            }),
            is_owner: false,
        };

        let mut result = sdk::ContractResult::new(init_state.clone());

        contract_logic(&context, &mut result);

        assert!(result.error.is_empty());
        assert_eq!(
            result.state.custom_types.get("UserObject").unwrap().clone(),
            Fields::Object(HashMap::from([("value".to_owned(), DynamicType::bool)]))
        );
        assert_eq!(
            result.state.custom_types.get("UserBasic").unwrap().clone(),
            Fields::Basic(Box::new(DynamicType::bool))
        );
        assert_eq!(result.state.custom_types.len(), 2);

        ////////////////////////////////////////////////////////////////
        // Unit process
        ////////////////////////////////////////////////////////////////
        let unit_process = UnitProcess {
            name: "Unit example".to_owned(),
            outputs: vec![
                Data {
                    name: "Example Object".to_owned(),
                    type_name: "UserObject".to_owned(),
                    content: json!({"value": false}),
                    targets: None,
                    metadata: None,
                },
                Data {
                    name: "Example bool".to_owned(),
                    type_name: "bool".to_owned(),
                    content: json!(true),
                    targets: None,
                    metadata: None,
                },
            ],
            inputs: vec![Data {
                name: "Example Basic".to_owned(),
                type_name: "UserBasic".to_owned(),
                content: json!(false),
                targets: None,
                metadata: None,
            }],
            properties: vec![],
        };

        let context = sdk::Context {
            event: Events::ChangeProductionSystem(ChangeProductionSystem::ModifyUnitProcess {
                delete: None,
                modify: None,
                add: Some(vec![unit_process]),
            }),
            is_owner: false,
        };

        let mut result = sdk::ContractResult::new(result.state);
        contract_logic(&context, &mut result);

        println!("{}", result.error);
        assert!(result.error.is_empty());

        assert_eq!(result.state.unit_process.len(), 1);
        assert_eq!(result.state.unit_process[0].name, "Unit example");
        assert_eq!(result.state.unit_process[0].outputs.len(), 2);

        let data = result.state.unit_process[0].outputs[0].clone();
        assert_eq!(data.name, "Example Object");
        assert_eq!(data.type_name, "UserObject");
        assert_eq!(data.content, json!({"value": false}));
        let data = result.state.unit_process[0].outputs[1].clone();
        assert_eq!(data.name, "Example bool");
        assert_eq!(data.type_name, "bool");
        assert_eq!(data.content, json!(true),);

        let data = result.state.unit_process[0].inputs[0].clone();
        assert_eq!(data.name, "Example Basic");
        assert_eq!(data.type_name, "UserBasic");
        assert_eq!(data.content, json!(false),);

        assert!(result.success);
    }

    #[test]
    fn test_dummy_fail() {
        ////////////////////////////////////////////////////////////////
        // Type definition
        ////////////////////////////////////////////////////////////////
        let init_state = ProductionSystem {
            name: "example".to_owned(),
            version: 1,
            unit_process: vec![],
            custom_types: HashMap::new(),
            properties: vec![],
        };

        let context = sdk::Context {
            event: Events::ChangeProductionSystem(ChangeProductionSystem::ModifyTypes {
                delete: None,
                add: Some(vec![(
                    "UserObject".to_owned(),
                    Fields::Object(HashMap::from([("value".to_owned(), DynamicType::Dummy)])),
                )]),
            }),
            is_owner: false,
        };

        let mut result = sdk::ContractResult::new(init_state.clone());

        contract_logic(&context, &mut result);

        assert!(!result.error.is_empty());
        assert!(!result.success);

        let context = sdk::Context {
            event: Events::ChangeProductionSystem(ChangeProductionSystem::ModifyTypes {
                delete: None,
                add: Some(vec![(
                    "UserBasic".to_owned(),
                    Fields::Basic(Box::new(DynamicType::Dummy)),
                )]),
            }),
            is_owner: false,
        };

        let mut result = sdk::ContractResult::new(init_state.clone());

        contract_logic(&context, &mut result);

        assert!(!result.error.is_empty());
        assert!(!result.success);

        ////////////////////////////////////////////////////////////////
        // Unit process
        ////////////////////////////////////////////////////////////////
        let unit_process = UnitProcess {
            name: "Unit example".to_owned(),
            outputs: vec![Data {
                name: "Example Dummy".to_owned(),
                type_name: "Dummy".to_owned(),
                content: json!({}),
                targets: None,
                metadata: None,
            }],
            inputs: vec![],
            properties: vec![],
        };

        let context = sdk::Context {
            event: Events::ChangeProductionSystem(ChangeProductionSystem::ModifyUnitProcess {
                delete: None,
                modify: None,
                add: Some(vec![unit_process]),
            }),
            is_owner: false,
        };

        let mut result = sdk::ContractResult::new(init_state.clone());
        contract_logic(&context, &mut result);

        assert!(!result.error.is_empty());
        assert!(!result.success);
    }

    #[test]
    fn test_option_fail() {
        ////////////////////////////////////////////////////////////////
        // Type definition
        ////////////////////////////////////////////////////////////////
        let init_state = ProductionSystem {
            name: "example".to_owned(),
            version: 1,
            unit_process: vec![],
            custom_types: HashMap::new(),
            properties: vec![],
        };

        let context = sdk::Context {
            event: Events::ChangeProductionSystem(ChangeProductionSystem::ModifyTypes {
                delete: None,
                add: Some(vec![(
                    "UserObject".to_owned(),
                    Fields::Object(HashMap::from([(
                        "value".to_owned(),
                        DynamicType::Option(Box::new(DynamicType::Dummy)),
                    )])),
                )]),
            }),
            is_owner: false,
        };

        let mut result = sdk::ContractResult::new(init_state.clone());

        contract_logic(&context, &mut result);

        assert!(!result.error.is_empty());
        assert!(!result.success);

        let context = sdk::Context {
            event: Events::ChangeProductionSystem(ChangeProductionSystem::ModifyTypes {
                delete: None,
                add: Some(vec![(
                    "UserBasic".to_owned(),
                    Fields::Basic(Box::new(DynamicType::Option(Box::new(DynamicType::String)))),
                )]),
            }),
            is_owner: false,
        };

        let mut result = sdk::ContractResult::new(init_state.clone());

        contract_logic(&context, &mut result);

        assert!(!result.error.is_empty());
        assert!(!result.success);

        ////////////////////////////////////////////////////////////////
        // Unit process
        ////////////////////////////////////////////////////////////////
        let unit_process = UnitProcess {
            name: "Unit example".to_owned(),
            outputs: vec![Data {
                name: "Example Dummy".to_owned(),
                type_name: "Option".to_owned(),
                content: json!({}),
                targets: None,
                metadata: None,
            }],
            inputs: vec![],
            properties: vec![],
        };

        let context = sdk::Context {
            event: Events::ChangeProductionSystem(ChangeProductionSystem::ModifyUnitProcess {
                delete: None,
                modify: None,
                add: Some(vec![unit_process]),
            }),
            is_owner: false,
        };

        let mut result = sdk::ContractResult::new(init_state.clone());
        contract_logic(&context, &mut result);

        assert!(!result.error.is_empty());
        assert!(!result.success);
    }

    // TODO Poner más ejemplos como Enum(Vec)...
    #[test]
    fn test_enum() {
        ////////////////////////////////////////////////////////////////
        // Type definition
        ////////////////////////////////////////////////////////////////
        let init_state = ProductionSystem {
            name: "example".to_owned(),
            version: 1,
            unit_process: vec![],
            custom_types: HashMap::new(),
            properties: vec![],
        };

        let context = sdk::Context {
            event: Events::ChangeProductionSystem(ChangeProductionSystem::ModifyTypes {
                delete: None,
                add: Some(vec![
                    (
                        "UserObject".to_owned(),
                        Fields::Object(HashMap::from([(
                            "value".to_owned(),
                            DynamicType::Enum(HashMap::from([
                                ("Name".to_owned(), DynamicType::Dummy),
                                ("Data".to_owned(), DynamicType::String),
                            ])),
                        )])),
                    ),
                    (
                        "UserBasic".to_owned(),
                        Fields::Basic(Box::new(DynamicType::Enum(HashMap::from([
                            ("Name".to_owned(), DynamicType::Dummy),
                            ("Data".to_owned(), DynamicType::String),
                        ])))),
                    ),
                ]),
            }),
            is_owner: false,
        };

        let mut result = sdk::ContractResult::new(init_state.clone());

        contract_logic(&context, &mut result);

        assert!(result.error.is_empty());
        assert_eq!(
            result.state.custom_types.get("UserObject").unwrap().clone(),
            Fields::Object(HashMap::from([(
                "value".to_owned(),
                DynamicType::Enum(HashMap::from([
                    ("Name".to_owned(), DynamicType::Dummy),
                    ("Data".to_owned(), DynamicType::String)
                ]))
            )]))
        );
        assert_eq!(
            result.state.custom_types.get("UserBasic").unwrap().clone(),
            Fields::Basic(Box::new(DynamicType::Enum(HashMap::from([
                ("Name".to_owned(), DynamicType::Dummy),
                ("Data".to_owned(), DynamicType::String)
            ]))))
        );
        assert_eq!(result.state.custom_types.len(), 2);

        ////////////////////////////////////////////////////////////////
        // Unit process
        ////////////////////////////////////////////////////////////////
        let unit_process = UnitProcess {
            name: "Unit example".to_owned(),
            outputs: vec![Data {
                name: "Example Object".to_owned(),
                type_name: "UserObject".to_owned(),
                content: json!({"value": {"Data": "info"}}),
                targets: None,
                metadata: None,
            }],
            inputs: vec![Data {
                name: "Example Basic".to_owned(),
                type_name: "UserBasic".to_owned(),
                content: json!("Name"),
                targets: None,
                metadata: None,
            }],
            properties: vec![],
        };

        let context = sdk::Context {
            event: Events::ChangeProductionSystem(ChangeProductionSystem::ModifyUnitProcess {
                delete: None,
                modify: None,
                add: Some(vec![unit_process]),
            }),
            is_owner: false,
        };

        let mut result = sdk::ContractResult::new(result.state);
        contract_logic(&context, &mut result);

        println!("{}", result.error);
        assert!(result.error.is_empty());

        assert_eq!(result.state.unit_process.len(), 1);
        assert_eq!(result.state.unit_process[0].name, "Unit example");
        assert_eq!(result.state.unit_process[0].outputs.len(), 1);

        let data = result.state.unit_process[0].outputs[0].clone();
        assert_eq!(data.name, "Example Object");
        assert_eq!(data.type_name, "UserObject");
        assert_eq!(data.content, json!({"value": {"Data": "info"}}));

        let data = result.state.unit_process[0].inputs[0].clone();
        assert_eq!(data.name, "Example Basic");
        assert_eq!(data.type_name, "UserBasic");
        assert_eq!(data.content, json!("Name"));

        assert!(result.success);

        ////////////////////////////////////////////////////////////////
        // Unit process register data
        ////////////////////////////////////////////////////////////////
        let context = sdk::Context {
            event: Events::RegisterData(vec![UnitData {
                name: "Unit example".to_owned(),
                outputs: Some(vec![RegisterData {
                    name: "Example Object".to_owned(),
                    type_name: "UserObject".to_owned(),
                    content: json!({"value": "Name"}),
                    targets: None,
                }]),
                inputs: Some(vec![RegisterData {
                    name: "Example Basic".to_owned(),
                    type_name: "UserBasic".to_owned(),
                    content: json!({"Data": "info"}),
                    targets: None,
                }]),
                properties: None,
            }]),
            is_owner: false,
        };

        let mut result = sdk::ContractResult::new(result.state);
        contract_logic(&context, &mut result);

        println!("{}", result.error);
        assert!(result.error.is_empty());

        assert_eq!(result.state.unit_process.len(), 1);
        assert_eq!(result.state.unit_process[0].name, "Unit example");
        assert_eq!(result.state.unit_process[0].outputs.len(), 1);

        let data = result.state.unit_process[0].outputs[0].clone();
        assert_eq!(data.name, "Example Object");
        assert_eq!(data.type_name, "UserObject");
        assert_eq!(data.content, json!({"value": "Name"}));

        let data = result.state.unit_process[0].inputs[0].clone();
        assert_eq!(data.name, "Example Basic");
        assert_eq!(data.type_name, "UserBasic");
        assert_eq!(data.content, json!({"Data": "info"}));

        assert!(result.success);
    }

    #[test]
    fn test_enum_fail() {
        ////////////////////////////////////////////////////////////////
        // Type definition
        ////////////////////////////////////////////////////////////////
        let init_state = ProductionSystem {
            name: "example".to_owned(),
            version: 1,
            unit_process: vec![],
            custom_types: HashMap::new(),
            properties: vec![],
        };

        let context = sdk::Context {
            event: Events::ChangeProductionSystem(ChangeProductionSystem::ModifyTypes {
                delete: None,
                add: Some(vec![(
                    "UserObject".to_owned(),
                    Fields::Object(HashMap::from([(
                        "value".to_owned(),
                        DynamicType::Enum(HashMap::from([
                            ("Name".to_owned(), DynamicType::Dummy),
                            (
                                "Data".to_owned(),
                                DynamicType::Option(Box::new(DynamicType::String)),
                            ),
                        ])),
                    )])),
                )]),
            }),
            is_owner: false,
        };

        let mut result = sdk::ContractResult::new(init_state.clone());

        contract_logic(&context, &mut result);

        assert!(!result.error.is_empty());
        assert!(!result.success);

        let context = sdk::Context {
            event: Events::ChangeProductionSystem(ChangeProductionSystem::ModifyTypes {
                delete: None,
                add: Some(vec![(
                    "UserBasic".to_owned(),
                    Fields::Basic(Box::new(DynamicType::Enum(HashMap::from([
                        ("Name".to_owned(), DynamicType::Dummy),
                        (
                            "Data".to_owned(),
                            DynamicType::Option(Box::new(DynamicType::String)),
                        ),
                    ])))),
                )]),
            }),
            is_owner: false,
        };

        let mut result = sdk::ContractResult::new(init_state.clone());

        contract_logic(&context, &mut result);

        assert!(!result.error.is_empty());
        assert!(!result.success);

        ////////////////////////////////////////////////////////////////
        // Unit process
        ////////////////////////////////////////////////////////////////
        let unit_process = UnitProcess {
            name: "Unit example".to_owned(),
            outputs: vec![Data {
                name: "Example Dummy".to_owned(),
                type_name: "Enum".to_owned(),
                content: json!({}),
                targets: None,
                metadata: None,
            }],
            inputs: vec![],
            properties: vec![],
        };

        let context = sdk::Context {
            event: Events::ChangeProductionSystem(ChangeProductionSystem::ModifyUnitProcess {
                delete: None,
                modify: None,
                add: Some(vec![unit_process]),
            }),
            is_owner: false,
        };

        let mut result = sdk::ContractResult::new(init_state.clone());
        contract_logic(&context, &mut result);

        assert!(!result.error.is_empty());
        assert!(!result.success);
    }

    // TODO Poner más ejemplos como Vec(Enum)...
    #[test]
    fn test_vec() {
        ////////////////////////////////////////////////////////////////
        // Type definition
        ////////////////////////////////////////////////////////////////
        let init_state = ProductionSystem {
            name: "example".to_owned(),
            version: 1,
            unit_process: vec![],
            custom_types: HashMap::new(),
            properties: vec![],
        };

        let context = sdk::Context {
            event: Events::ChangeProductionSystem(ChangeProductionSystem::ModifyTypes {
                delete: None,
                add: Some(vec![
                    (
                        "UserObject".to_owned(),
                        Fields::Object(HashMap::from([(
                            "value".to_owned(),
                            DynamicType::Vec(Box::new(DynamicType::String)),
                        )])),
                    ),
                    (
                        "UserBasic".to_owned(),
                        Fields::Basic(Box::new(DynamicType::Vec(Box::new(DynamicType::u64)))),
                    ),
                ]),
            }),
            is_owner: false,
        };

        let mut result = sdk::ContractResult::new(init_state.clone());

        contract_logic(&context, &mut result);

        assert!(result.error.is_empty());
        assert_eq!(
            result.state.custom_types.get("UserObject").unwrap().clone(),
            Fields::Object(HashMap::from([(
                "value".to_owned(),
                DynamicType::Vec(Box::new(DynamicType::String))
            )]))
        );
        assert_eq!(
            result.state.custom_types.get("UserBasic").unwrap().clone(),
            Fields::Basic(Box::new(DynamicType::Vec(Box::new(DynamicType::u64)))),
        );
        assert_eq!(result.state.custom_types.len(), 2);

        ////////////////////////////////////////////////////////////////
        // Unit process
        ////////////////////////////////////////////////////////////////
        let unit_process = UnitProcess {
            name: "Unit example".to_owned(),
            outputs: vec![Data {
                name: "Example Object".to_owned(),
                type_name: "UserObject".to_owned(),
                content: json!({"value": ["one", "two"]}),
                targets: None,
                metadata: None,
            }],
            inputs: vec![Data {
                name: "Example Basic".to_owned(),
                type_name: "UserBasic".to_owned(),
                content: json!([0, 1, 2, 3, 4, 5]),
                targets: None,
                metadata: None,
            }],
            properties: vec![],
        };

        let context = sdk::Context {
            event: Events::ChangeProductionSystem(ChangeProductionSystem::ModifyUnitProcess {
                delete: None,
                modify: None,
                add: Some(vec![unit_process]),
            }),
            is_owner: false,
        };

        let mut result = sdk::ContractResult::new(result.state);
        contract_logic(&context, &mut result);

        println!("{}", result.error);
        assert!(result.error.is_empty());

        assert_eq!(result.state.unit_process.len(), 1);
        assert_eq!(result.state.unit_process[0].name, "Unit example");
        assert_eq!(result.state.unit_process[0].outputs.len(), 1);

        let data = result.state.unit_process[0].outputs[0].clone();
        assert_eq!(data.name, "Example Object");
        assert_eq!(data.type_name, "UserObject");
        assert_eq!(data.content, json!({"value": ["one", "two"]}));

        let data = result.state.unit_process[0].inputs[0].clone();
        assert_eq!(data.name, "Example Basic");
        assert_eq!(data.type_name, "UserBasic");
        assert_eq!(data.content, json!([0, 1, 2, 3, 4, 5]));

        assert!(result.success);
    }

    #[test]
    fn test_vec_fail() {
        ////////////////////////////////////////////////////////////////
        // Type definition
        ////////////////////////////////////////////////////////////////
        let init_state = ProductionSystem {
            name: "example".to_owned(),
            version: 1,
            unit_process: vec![],
            custom_types: HashMap::new(),
            properties: vec![],
        };

        let context = sdk::Context {
            event: Events::ChangeProductionSystem(ChangeProductionSystem::ModifyTypes {
                delete: None,
                add: Some(vec![(
                    "UserObject".to_owned(),
                    Fields::Object(HashMap::from([(
                        "value".to_owned(),
                        DynamicType::Vec(Box::new(DynamicType::Dummy)),
                    )])),
                )]),
            }),
            is_owner: false,
        };

        let mut result = sdk::ContractResult::new(init_state.clone());

        contract_logic(&context, &mut result);

        assert!(!result.error.is_empty());
        assert!(!result.success);

        let context = sdk::Context {
            event: Events::ChangeProductionSystem(ChangeProductionSystem::ModifyTypes {
                delete: None,
                add: Some(vec![(
                    "UserBasic".to_owned(),
                    Fields::Basic(Box::new(DynamicType::Vec(Box::new(DynamicType::Dummy)))),
                )]),
            }),
            is_owner: false,
        };

        let mut result = sdk::ContractResult::new(init_state.clone());

        contract_logic(&context, &mut result);

        assert!(!result.error.is_empty());
        assert!(!result.success);

        ////////////////////////////////////////////////////////////////
        ////////////////////////////////////////////////////////////////

        let init_state = ProductionSystem {
            name: "example".to_owned(),
            version: 1,
            unit_process: vec![],
            custom_types: HashMap::new(),
            properties: vec![],
        };

        let context = sdk::Context {
            event: Events::ChangeProductionSystem(ChangeProductionSystem::ModifyTypes {
                delete: None,
                add: Some(vec![(
                    "UserObject".to_owned(),
                    Fields::Object(HashMap::from([(
                        "value".to_owned(),
                        DynamicType::Vec(Box::new(DynamicType::Option(Box::new(
                            DynamicType::String,
                        )))),
                    )])),
                )]),
            }),
            is_owner: false,
        };

        let mut result = sdk::ContractResult::new(init_state.clone());

        contract_logic(&context, &mut result);

        assert!(!result.error.is_empty());
        assert!(!result.success);

        let context = sdk::Context {
            event: Events::ChangeProductionSystem(ChangeProductionSystem::ModifyTypes {
                delete: None,
                add: Some(vec![(
                    "UserBasic".to_owned(),
                    Fields::Basic(Box::new(DynamicType::Vec(Box::new(DynamicType::Option(
                        Box::new(DynamicType::String),
                    ))))),
                )]),
            }),
            is_owner: false,
        };

        let mut result = sdk::ContractResult::new(init_state.clone());

        contract_logic(&context, &mut result);

        assert!(!result.error.is_empty());
        assert!(!result.success);

        ////////////////////////////////////////////////////////////////
        // Unit process
        ////////////////////////////////////////////////////////////////
        let unit_process = UnitProcess {
            name: "Unit example".to_owned(),
            outputs: vec![Data {
                name: "Example Dummy".to_owned(),
                type_name: "Vec".to_owned(),
                content: json!([]),
                targets: None,
                metadata: None,
            }],
            inputs: vec![],
            properties: vec![],
        };

        let context = sdk::Context {
            event: Events::ChangeProductionSystem(ChangeProductionSystem::ModifyUnitProcess {
                delete: None,
                modify: None,
                add: Some(vec![unit_process]),
            }),
            is_owner: false,
        };

        let mut result = sdk::ContractResult::new(init_state.clone());
        contract_logic(&context, &mut result);

        assert!(!result.error.is_empty());
        assert!(!result.success);
    }

    #[test]
    fn test_type() {
        ////////////////////////////////////////////////////////////////
        // Type definition
        ////////////////////////////////////////////////////////////////
        let init_state = ProductionSystem {
            name: "example".to_owned(),
            version: 1,
            unit_process: vec![],
            custom_types: HashMap::new(),
            properties: vec![],
        };

        let context = sdk::Context {
            event: Events::ChangeProductionSystem(ChangeProductionSystem::ModifyTypes {
                delete: None,
                add: Some(vec![
                    (
                        "DataType".to_owned(),
                        Fields::Object(HashMap::from([
                            ("text".to_owned(), DynamicType::String),
                            ("value".to_owned(), DynamicType::u64),
                        ])),
                    ),
                    (
                        "VecType".to_owned(),
                        Fields::Basic(Box::new(DynamicType::Vec(Box::new(DynamicType::String)))),
                    ),
                    (
                        "UserObject".to_owned(),
                        Fields::Object(HashMap::from([(
                            "value".to_owned(),
                            DynamicType::Type("DataType".to_owned()),
                        )])),
                    ),
                    (
                        "UserVec".to_owned(),
                        Fields::Object(HashMap::from([(
                            "value".to_owned(),
                            DynamicType::Type("VecType".to_owned()),
                        )])),
                    ),
                ]),
            }),
            is_owner: false,
        };

        let mut result = sdk::ContractResult::new(init_state.clone());

        contract_logic(&context, &mut result);

        assert!(result.error.is_empty());
        assert_eq!(
            result.state.custom_types.get("VecType").unwrap().clone(),
            Fields::Basic(Box::new(DynamicType::Vec(Box::new(DynamicType::String)))),
        );
        assert_eq!(
            result.state.custom_types.get("UserVec").unwrap().clone(),
            Fields::Object(HashMap::from([(
                "value".to_owned(),
                DynamicType::Type("VecType".to_owned()),
            )])),
        );
        assert_eq!(
            result.state.custom_types.get("UserObject").unwrap().clone(),
            Fields::Object(HashMap::from([(
                "value".to_owned(),
                DynamicType::Type("DataType".to_owned())
            )]))
        );
        assert_eq!(
            result.state.custom_types.get("DataType").unwrap().clone(),
            Fields::Object(HashMap::from([
                ("text".to_owned(), DynamicType::String,),
                ("value".to_owned(), DynamicType::u64,)
            ])),
        );
        assert_eq!(result.state.custom_types.len(), 4);

        ////////////////////////////////////////////////////////////////
        // Unit process
        ////////////////////////////////////////////////////////////////
        let unit_process = UnitProcess {
            name: "Unit example".to_owned(),
            outputs: vec![Data {
                name: "Example Object".to_owned(),
                type_name: "UserObject".to_owned(),
                content: json!({"value": {"text": "info", "value": 30}}),
                targets: None,
                metadata: None,
            }],
            inputs: vec![Data {
                name: "Example Vec".to_owned(),
                type_name: "UserVec".to_owned(),
                content: json!({"value": ["one", "two"]}),
                targets: None,
                metadata: None,
            }],
            properties: vec![],
        };

        let context = sdk::Context {
            event: Events::ChangeProductionSystem(ChangeProductionSystem::ModifyUnitProcess {
                delete: None,
                modify: None,
                add: Some(vec![unit_process]),
            }),
            is_owner: false,
        };

        let mut result = sdk::ContractResult::new(result.state);
        contract_logic(&context, &mut result);

        println!("{}", result.error);
        assert!(result.error.is_empty());

        assert_eq!(result.state.unit_process.len(), 1);
        assert_eq!(result.state.unit_process[0].name, "Unit example");
        assert_eq!(result.state.unit_process[0].outputs.len(), 1);

        let data = result.state.unit_process[0].outputs[0].clone();
        assert_eq!(data.name, "Example Object");
        assert_eq!(data.type_name, "UserObject");
        assert_eq!(
            data.content,
            json!({"value": {"text": "info", "value": 30}})
        );

        assert_eq!(result.state.unit_process[0].inputs.len(), 1);

        let data = result.state.unit_process[0].inputs[0].clone();
        assert_eq!(data.name, "Example Vec");
        assert_eq!(data.type_name, "UserVec");
        assert_eq!(data.content, json!({"value": ["one", "two"]}));

        assert!(result.success);
    }

    #[test]
    fn test_type_fail() {
        ////////////////////////////////////////////////////////////////
        // Type definition
        ////////////////////////////////////////////////////////////////
        let init_state = ProductionSystem {
            name: "example".to_owned(),
            version: 1,
            unit_process: vec![],
            custom_types: HashMap::new(),
            properties: vec![],
        };

        ////////////////////////////////////////////////////////////////
        // Unit process
        ////////////////////////////////////////////////////////////////
        let unit_process = UnitProcess {
            name: "Unit example".to_owned(),
            outputs: vec![Data {
                name: "Example Dummy".to_owned(),
                type_name: "Type".to_owned(),
                content: json!({}),
                targets: None,
                metadata: None,
            }],
            inputs: vec![],
            properties: vec![],
        };

        let context = sdk::Context {
            event: Events::ChangeProductionSystem(ChangeProductionSystem::ModifyUnitProcess {
                delete: None,
                modify: None,
                add: Some(vec![unit_process]),
            }),
            is_owner: false,
        };

        let mut result = sdk::ContractResult::new(init_state.clone());
        contract_logic(&context, &mut result);

        assert!(!result.error.is_empty());
        assert!(!result.success);
    }

    // TODO REHACER
    #[test]
    fn change_operation_name() {
        let init_state = ProductionSystem {
            name: "wine process".to_owned(),
            version: 1,
            unit_process: vec![],
            custom_types: HashMap::new(),
            properties: vec![],
        };

        let context = sdk::Context {
            event: Events::ChangeProductionSystem(ChangeProductionSystem::ModifyProductionSystem {
                name: Some("wine process 2".to_owned()),
                delete_properties: None,
                modify_properties: None,
                add_properties: None,
            }),
            is_owner: false,
        };

        let mut result = sdk::ContractResult::new(init_state);
        contract_logic(&context, &mut result);

        assert_eq!(result.state.version, 2);
        assert_eq!(result.state.name, "wine process 2");
        assert!(result.success);
    }

    #[test]
    fn test_change_operation_init() {
        let init_state = ProductionSystem {
            name: "".to_owned(),
            version: 0,
            unit_process: vec![],
            custom_types: HashMap::new(),
            properties: vec![],
        };

        let context = sdk::Context {
            event: Events::ChangeProductionSystem(ChangeProductionSystem::Init {
                name: "wine process".to_owned(),
                unit_process: Some(vec![UnitProcess {
                    name: "Unit example".to_owned(),
                    outputs: vec![Data {
                        name: "Example Object".to_owned(),
                        type_name: "UserObject".to_owned(),
                        content: json!({"name": "ExampleName"}),
                        targets: None,
                        metadata: None,
                    }],
                    inputs: vec![Data {
                        name: "Example Basic".to_owned(),
                        type_name: "UserBasic".to_owned(),
                        content: json!("ExampleBasic"),
                        targets: None,
                        metadata: None,
                    }],
                    properties: vec![Properties {
                        name: "Example String".to_owned(),
                        type_name: "String".to_owned(),
                        content: json!("ExampleString"),
                    }],
                }]),
                properties: Some(vec![Properties {
                    name: "Example Object".to_owned(),
                    type_name: "UserObject".to_owned(),
                    content: json!({"name": "ExampleName"}),
                }]),
                types: Some(vec![
                    (
                        "UserObject".to_owned(),
                        Fields::Object(HashMap::from([("name".to_owned(), DynamicType::String)])),
                    ),
                    (
                        "UserBasic".to_owned(),
                        Fields::Basic(Box::new(DynamicType::String)),
                    ),
                ]),
            }),
            is_owner: false,
        };

        let mut result = sdk::ContractResult::new(init_state);
        contract_logic(&context, &mut result);

        assert_eq!(result.state.name, "wine process");
        assert_eq!(result.state.version, 1);
        assert_eq!(result.state.properties.len(), 1);
        assert_eq!(
            result.state.properties[0],
            Properties {
                name: "Example Object".to_owned(),
                type_name: "UserObject".to_owned(),
                content: json!({"name": "ExampleName"}),
            }
        );
        assert_eq!(result.state.unit_process.len(), 1);
        assert_eq!(
            result.state.unit_process[0],
            UnitProcess {
                name: "Unit example".to_owned(),
                outputs: vec![Data {
                    name: "Example Object".to_owned(),
                    type_name: "UserObject".to_owned(),
                    content: json!({"name": "ExampleName"}),
                    targets: None,
                    metadata: None
                },],
                inputs: vec![Data {
                    name: "Example Basic".to_owned(),
                    type_name: "UserBasic".to_owned(),
                    content: json!("ExampleBasic"),
                    targets: None,
                    metadata: None
                }],
                properties: vec![Properties {
                    name: "Example String".to_owned(),
                    type_name: "String".to_owned(),
                    content: json!("ExampleString"),
                }],
            }
        );
        assert_eq!(
            result.state.custom_types.get("UserObject").unwrap().clone(),
            Fields::Object(HashMap::from([("name".to_owned(), DynamicType::String)]))
        );
        assert_eq!(
            result.state.custom_types.get("UserBasic").unwrap().clone(),
            Fields::Basic(Box::new(DynamicType::String))
        );
        assert!(result.success);
    }

    #[test]
    fn test_add_change_delete() {
        ////////////////////////////////////////////////////////////////
        // Type definition
        ////////////////////////////////////////////////////////////////
        let init_state = ProductionSystem {
            name: "example".to_owned(),
            version: 1,
            unit_process: vec![],
            custom_types: HashMap::new(),
            properties: vec![],
        };

        let context = sdk::Context {
            event: Events::ChangeProductionSystem(ChangeProductionSystem::ModifyTypes {
                delete: None,
                add: Some(vec![
                    (
                        "UserObject".to_owned(),
                        Fields::Object(HashMap::from([("name".to_owned(), DynamicType::String)])),
                    ),
                    (
                        "UserBasic".to_owned(),
                        Fields::Basic(Box::new(DynamicType::String)),
                    ),
                ]),
            }),
            is_owner: false,
        };

        let mut result = sdk::ContractResult::new(init_state.clone());

        contract_logic(&context, &mut result);

        assert!(result.error.is_empty());
        assert_eq!(
            result.state.custom_types.get("UserObject").unwrap().clone(),
            Fields::Object(HashMap::from([("name".to_owned(), DynamicType::String)]))
        );
        assert_eq!(
            result.state.custom_types.get("UserBasic").unwrap().clone(),
            Fields::Basic(Box::new(DynamicType::String))
        );
        assert_eq!(result.state.custom_types.len(), 2);

        ////////////////////////////////////////////////////////////////
        // Unit process add
        ////////////////////////////////////////////////////////////////
        let unit_process = UnitProcess {
            name: "Unit example".to_owned(),
            outputs: vec![
                Data {
                    name: "Example Object".to_owned(),
                    type_name: "UserObject".to_owned(),
                    content: json!({"name": "ExampleName"}),
                    targets: None,
                    metadata: None,
                },
                Data {
                    name: "Example String".to_owned(),
                    type_name: "String".to_owned(),
                    content: json!("ExampleString"),
                    targets: None,
                    metadata: None,
                },
            ],
            inputs: vec![Data {
                name: "Example Basic".to_owned(),
                type_name: "UserBasic".to_owned(),
                content: json!("ExampleBasic"),
                targets: None,
                metadata: None,
            }],
            properties: vec![],
        };

        let context = sdk::Context {
            event: Events::ChangeProductionSystem(ChangeProductionSystem::ModifyUnitProcess {
                delete: None,
                modify: None,
                add: Some(vec![unit_process]),
            }),
            is_owner: false,
        };

        let mut result = sdk::ContractResult::new(result.state);
        contract_logic(&context, &mut result);

        println!("{}", result.error);
        assert!(result.error.is_empty());

        assert_eq!(result.state.unit_process.len(), 1);
        assert_eq!(result.state.unit_process[0].name, "Unit example");
        assert_eq!(result.state.unit_process[0].outputs.len(), 2);

        let data = result.state.unit_process[0].outputs[0].clone();
        assert_eq!(data.name, "Example Object");
        assert_eq!(data.type_name, "UserObject");
        assert_eq!(data.content, json!({"name": "ExampleName"}));
        let data = result.state.unit_process[0].outputs[1].clone();
        assert_eq!(data.name, "Example String");
        assert_eq!(data.type_name, "String");
        assert_eq!(data.content, json!("ExampleString"));

        let data = result.state.unit_process[0].inputs[0].clone();
        assert_eq!(data.name, "Example Basic");
        assert_eq!(data.type_name, "UserBasic");
        assert_eq!(data.content, json!("ExampleBasic"));

        assert!(result.success);

        ////////////////////////////////////////////////////////////////
        // Unit process modify
        ////////////////////////////////////////////////////////////////
        let unit_process = UnitProcess {
            name: "Unit example modify".to_owned(),
            outputs: vec![Data {
                name: "Example Basic modify".to_owned(),
                type_name: "UserBasic".to_owned(),
                content: json!("ExampleBasic"),
                targets: None,
                metadata: None,
            }],
            inputs: vec![Data {
                name: "Example Object modify".to_owned(),
                type_name: "UserObject".to_owned(),
                content: json!({"name": "ExampleName"}),
                targets: None,
                metadata: None,
            }],
            properties: vec![],
        };

        let context = sdk::Context {
            event: Events::ChangeProductionSystem(ChangeProductionSystem::ModifyUnitProcess {
                delete: None,
                modify: Some(vec![("Unit example".to_owned(), unit_process)]),
                add: None,
            }),
            is_owner: false,
        };

        let mut result = sdk::ContractResult::new(result.state);
        contract_logic(&context, &mut result);

        println!("{}", result.error);
        assert!(result.error.is_empty());

        assert_eq!(result.state.unit_process.len(), 1);
        assert_eq!(result.state.unit_process[0].name, "Unit example modify");
        assert_eq!(result.state.unit_process[0].outputs.len(), 1);

        let data = result.state.unit_process[0].outputs[0].clone();
        assert_eq!(data.name, "Example Basic modify");
        assert_eq!(data.type_name, "UserBasic");
        assert_eq!(data.content, json!("ExampleBasic"));

        let data = result.state.unit_process[0].inputs[0].clone();
        assert_eq!(data.name, "Example Object modify");
        assert_eq!(data.type_name, "UserObject");
        assert_eq!(data.content, json!({"name": "ExampleName"}));

        ////////////////////////////////////////////////////////////////
        // Unit process modify
        ////////////////////////////////////////////////////////////////

        let context = sdk::Context {
            event: Events::ChangeProductionSystem(ChangeProductionSystem::ModifyUnitProcess {
                delete: Some(vec!["Unit example modify".to_owned()]),
                modify: None,
                add: None,
            }),
            is_owner: false,
        };

        let mut result = sdk::ContractResult::new(result.state);
        contract_logic(&context, &mut result);

        assert!(result.error.is_empty());
        assert_eq!(result.state.unit_process.len(), 0);
        assert!(result.success);
    }
}
