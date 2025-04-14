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
        return true; // Se encontró un ciclo
    }
    if visited.contains(node) {
        return false; // Ya fue procesado sin ciclo
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

pub fn check_cycle(cycle_types: HashMap<String, Vec<String>>) -> Result<(), String> {
    let mut visited = HashSet::new();
    let mut stack = HashSet::new();

    for type_name in cycle_types.keys() {
        if !visited.contains(type_name)
            && has_cycle(type_name, &cycle_types, &mut visited, &mut stack)
        {
            return Err(format!("A Cycle is detected in {} type", type_name));
        }
    }

    Ok(())
}
/// Define the state of the contract.
#[derive(Serialize, Deserialize, Clone, Debug)]
struct ProductionSystem {
    pub name: String,
    pub custom_types: HashMap<String, Fields>,
    pub version: u32,
    pub unit_process: Vec<UnitProcess>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Fields(pub HashMap<String, DynamicType>);

impl Fields {
    fn count_options(&self) -> (usize, usize) {
        let mut count = 0;
        for (_, c_type) in self.0.clone() {
            if c_type.is_option() {
                count += 1;
            }
        }

        (self.0.len(), count)
    }

    fn check_data(&self, custom_types: HashMap<String, Fields>) -> Result<Vec<String>, String> {
        if self.0.is_empty() {
            return Err("Fields can not be empty".to_owned());
        }

        let mut internal_types: Vec<String> = vec![];

        for (field, c_type) in self.0.iter() {
            if field.is_empty() {
                return Err("Field can not be empty".to_owned());
            }
            c_type.check_data(custom_types.clone(), &mut internal_types)?;
        }

        Ok(internal_types)
    }

    fn check_value(
        &self,
        data: Value,
        custom_types: HashMap<String, Fields>,
    ) -> Result<(), String> {
        let Some(mut data_object) = data.as_object().cloned() else {
            return Err("Data can not convert in Object".to_owned());
        };

        let (len, options) = self.count_options();
        if data_object.len() < len - options || data_object.len() > len {
            return Err("Data fields and type fields is not the same".to_owned());
        }

        for (custom_type_name, custom_type_type) in self.0.clone() {
            if let Some(field_type) = data_object.remove(&custom_type_name) {
                custom_type_type.deserialize(field_type, custom_types.clone())?;
            } else {
                if !custom_type_type.is_option() {
                    return Err(format!(
                        "A field in type do not exist in data: {}",
                        custom_type_name
                    ));
                }
            };
        }

        if !data_object.is_empty() {
            return Err("Data has more fields than type".to_owned());
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
    Enum(Fields),
    Type(String),
    Option(Box<DynamicType>),
}

impl DynamicType {
    fn is_option(&self) -> bool {
        match self {
            DynamicType::Option(_) => true,
            _ => false,
        }
    }

    fn deserialize(
        &self,
        value: Value,
        custom_types: HashMap<String, Fields>,
    ) -> Result<(), String> {
        match self {
            DynamicType::String => {
                if value.as_str().is_none() {
                    return Err("Can not deserialize Value as String".to_owned());
                }
            }
            DynamicType::u64 => {
                if value.as_u64().is_none() {
                    return Err("Can not deserialize Value as u64".to_owned());
                }
            }
            DynamicType::f64 => {
                if value.as_f64().is_none() {
                    return Err("Can not deserialize Value as f64".to_owned());
                }
            }
            DynamicType::i64 => {
                if value.as_i64().is_none() {
                    return Err("Can not deserialize Value as i64".to_owned());
                }
            }
            DynamicType::bool => {
                if value.as_bool().is_none() {
                    return Err("Can not deserialize Value as bool".to_owned());
                }
            }
            DynamicType::Vec(vec_type) => {
                let Some(vec_dynamic) = value.as_array() else {
                    return Err("Can not deserialize Value as Vec".to_owned());
                };
                for val in vec_dynamic.clone() {
                    vec_type.deserialize(val, custom_types.clone())?;
                }
            }
            DynamicType::Enum(enum_type) => {
                let Some(obj_dynamic) = value.as_object().cloned() else {
                    return Err("Can not deserialize Value as Enum".to_owned());
                };

                if obj_dynamic.len() != 1 {
                    return Err("Can not deserialize, in Enum Object must have one field".to_owned());
                }

                for (value_name, value_val) in obj_dynamic {
                    let Some(type_dyn) = enum_type.0.get(&value_name) else {
                        return Err("Can not deserialize, Value does not match enum".to_owned());
                    };

                    type_dyn.deserialize(value_val, custom_types.clone())?;
                }
            }
            DynamicType::Type(c_type) => {
                let Some(mut obj_dynamic) = value.as_object().cloned() else {
                    return Err("Can not deserialize Value as Object".to_owned());
                };

                let Some(obj_type) = custom_types.get(c_type) else {
                    return Err(format!(
                        "Can not deserialize, Custom type {} does not exist",
                        c_type
                    ));
                };

                let (len, options) = obj_type.count_options();
                if obj_dynamic.len() < len - options || obj_dynamic.len() > len {
                    return Err(
                        "Can not deserialize, Value has diferents fields than Object".to_owned(),
                    );
                }


                for (type_field, type_dyn) in obj_type.0.clone() {
                    if let Some(value) = obj_dynamic.remove(&type_field) {
                        type_dyn.deserialize(value, custom_types.clone())?;
                    } else {
                        if !type_dyn.is_option() {
                            return Err(format!(
                                "Can not deserialize, Value has not {} field",
                                type_field
                            ));
                        }
                    };
                }

                if !obj_dynamic.is_empty() {
                    return Err("Can not deserialize, Value has more fields than Type".to_owned());
                }
            }
            DynamicType::Option(option) => {
                if value.is_null() {
                    return Ok(());
                } else {
                    return option.deserialize(value, custom_types);
                }
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
            DynamicType::Vec(vec_type) => {
                vec_type.check_data(custom_types, internal_types)?;
            }
            DynamicType::Enum(enum_type) => {
                for (type_field, type_dyn) in enum_type.0.clone() {
                    if type_field.is_empty() {
                        return Err("Field can not be empty".to_owned());
                    }

                    type_dyn.check_data(custom_types.clone(), internal_types)?;
                }
            }
            DynamicType::Type(c_type) => {
                if c_type.is_empty() {
                    return Err(format!("Custom type con not be empty"));
                }

                if custom_types.get(c_type).is_none() {
                    return Err(format!("Type {} does not exist", c_type));
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
    pub main_outputs: Option<Vec<Data>>,
    pub inputs: Option<Vec<Data>>,
    pub other_outputs: Option<Vec<Data>>,
    pub scope: Option<Vec<Scopes>>,
}

// Define the process of the production system with the elements.
#[derive(Serialize, Deserialize, Clone, Debug)]
struct UnitProcess {
    pub name: String,
    pub main_outputs: Vec<Data>,
    pub inputs: Vec<Data>,
    pub other_outputs: Vec<Data>,
    pub scope: Vec<Scopes>,
}

impl UnitProcess {
    pub fn check_data(&self, custom_types: HashMap<String, Fields>) -> Result<(), String> {
        let mut names = vec![];

        for m_o in self.main_outputs.clone() {
            m_o.check_data(custom_types.clone())?;
            names.push(m_o.name);
        }

        for i in self.inputs.clone() {
            i.check_data(custom_types.clone())?;
            names.push(i.name);
        }

        for o_o in self.other_outputs.clone() {
            o_o.check_data(custom_types.clone())?;
            names.push(o_o.name);
        }

        let hash_name: HashSet<String> = HashSet::from_iter(names.iter().cloned());

        if hash_name.len() != self.main_outputs.len() + self.inputs.len() + self.other_outputs.len()
        {
            return Err("inputs, main outputs and other outputs has not unique names".to_owned());
        }

        Ok(())
    }

    pub fn register_data(
        &mut self,
        unit: UnitData,
        custom_types: HashMap<String, Fields>,
    ) -> Result<(), String> {
        if unit.inputs.is_none() && unit.main_outputs.is_none() && unit.other_outputs.is_none() {
            return Err("Inputs, other outputs and main outputs are empty".to_owned());
        }

        if let Some(inputs) = unit.inputs {
            if inputs.is_empty() {
                return Err("Inputs can not be Some and be empty".to_owned());
            }

            let mut updates: usize = 0;

            for element_state in self.inputs.iter_mut() {
                for element_unit in inputs.clone() {
                    if element_state.name == element_unit.name {
                        element_state.register_data(element_unit, custom_types.clone())?;
                        updates += 1;
                    }
                }
            }

            if updates != inputs.len() {
                return Err(
                    "An attempt was made to update inputs that do not exist in the unit process"
                        .to_owned(),
                );
            }
        }

        if let Some(main_outputs) = unit.main_outputs {
            if main_outputs.is_empty() {
                return Err("Main outputs can not be Some and be empty".to_owned());
            }

            let mut updates: usize = 0;

            for element_state in self.main_outputs.iter_mut() {
                for element_unit in main_outputs.clone() {
                    if element_state.name == element_unit.name {
                        element_state.register_data(element_unit, custom_types.clone())?;
                        updates += 1;
                    }
                }
            }

            if updates != main_outputs.len() {
                return Err("An attempt was made to update main outputs that do not exist in the unit process".to_owned());
            }
        }

        if let Some(other_outputs) = unit.other_outputs {
            if other_outputs.is_empty() {
                return Err("Other outputs can not be Some and be empty".to_owned());
            }

            let mut updates: usize = 0;

            for element_state in self.other_outputs.iter_mut() {
                for element_unit in other_outputs.clone() {
                    if element_state.name == element_unit.name {
                        element_state.register_data(element_unit, custom_types.clone())?;
                        updates += 1;
                    }
                }
            }

            if updates != other_outputs.len() {
                return Err("An attempt was made to update other outputs that do not exist in the unit process".to_owned());
            }
        }

        if let Some(scopes) = unit.scope {
            self.scope = scopes;
        }
        
        Ok(())
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct Data {
    name: String,
    type_name: String,
    content: Value,
}

impl Data {
    fn check_data(&self, custom_types: HashMap<String, Fields>) -> Result<(), String> {
        if self.name.is_empty() {
            return Err("Element name can not be empty".to_owned());
        }

        if self.type_name.is_empty() {
            return Err("Element type name can not be empty".to_owned());
        }

        if let Some(dynamic_type) = custom_types.get(&self.type_name) {
            dynamic_type.check_value(self.content.clone(), custom_types.clone())
        } else {
            match self.type_name.as_str() {
                "String" => DynamicType::String.deserialize(self.content.clone(), custom_types),
                "i64" => DynamicType::i64.deserialize(self.content.clone(), custom_types),
                "u64" => DynamicType::u64.deserialize(self.content.clone(), custom_types),
                "f64" => DynamicType::f64.deserialize(self.content.clone(), custom_types),
                "bool" => DynamicType::bool.deserialize(self.content.clone(), custom_types),
                _ => Err("Element type name can not be empty".to_owned()),
            }
        }
    }

    fn register_data(
        &mut self,
        data: Self,
        custom_types: HashMap<String, Fields>,
    ) -> Result<(), String> {
        if self.name != data.name {
            return Err("Data name is not the same".to_owned());
        }

        if self.type_name != data.type_name {
            return Err("Data type is not the same".to_owned());
        }

        if let Some(c_type) = custom_types.get(&self.type_name) {
            c_type.check_value(data.content.clone(), custom_types.clone())?;
        } else {
            match self.type_name.as_str() {
                "String" => DynamicType::String.deserialize(data.content.clone(), custom_types)?,
                "i64" => DynamicType::i64.deserialize(data.content.clone(), custom_types)?,
                "u64" => DynamicType::u64.deserialize(data.content.clone(), custom_types)?,
                "f64" => DynamicType::f64.deserialize(data.content.clone(), custom_types)?,
                "bool" => DynamicType::bool.deserialize(data.content.clone(), custom_types)?,
                _ => return Err("Element type name can not be empty".to_owned()),
            };
        };

        self.content = data.content;

        Ok(())
    }
}

// Context of the element or unit process.
#[derive(Serialize, Deserialize, Clone, Debug)]
enum Scopes {
    Temporal(u64),
    TimeZone(String),
    Tag(String)
}

// Define the events of the contract.
#[derive(Serialize, Deserialize, Clone)]
enum Events {
    ChangeTypes(ChangeTypes),
    ChangeProductionSystem(ChangeProductionSystem),
    RegisterData(UnitData),
}

#[derive(Serialize, Deserialize, Clone)]
enum ChangeTypes {
    Add { types: Vec<(String, Fields)> },
    Delete { names: Vec<String> },
}

// Operations to change the state of the contract.
#[derive(Serialize, Deserialize, Clone)]
enum ChangeProductionSystem {
    Init {
        name: String,
        unit_process: Vec<UnitProcess>,
    },
    NewName {
        name: String,
    },
    Modify {
        delete: Vec<String>,
        modification: Vec<(String, UnitProcess)>,
        add: Vec<UnitProcess>,
    },
}

#[unsafe(no_mangle)]
pub unsafe fn main_function(state_ptr: i32, event_ptr: i32, is_owner: i32) -> u32 {
    sdk::execute_contract(state_ptr, event_ptr, is_owner, contract_logic)
}

#[unsafe(no_mangle)]
pub unsafe fn init_check_function(state_ptr: i32) -> u32 {
    sdk::check_init_data(state_ptr, init_logic)
}

fn init_logic(_state: &ProductionSystem, contract_result: &mut sdk::ContractInitCheck) {
    contract_result.success = true;
}

fn contract_logic(
    context: &sdk::Context<ProductionSystem, Events>,
    contract_result: &mut sdk::ContractResult<ProductionSystem>,
) {
    let state = &mut contract_result.final_state;
    match context.event.clone() {
        Events::ChangeTypes ( operation ) => {
            if state.version == 0 {
                contract_result.error = "The first event must be Init event".to_owned();
                return;
            }

            match operation {
                ChangeTypes::Add { types } => {
                    let mut temporal_types = state.custom_types.clone();

                    for (name, fields) in types.clone() {
                        temporal_types.insert(name, fields);
                    }

                    let mut cycle_types: HashMap<String, Vec<String>> = HashMap::new();

                    for (name, fields) in temporal_types.clone() {
                        if name.is_empty() {
                            contract_result.error = "Type name can not be empty".to_owned();
                            return;
                        }

                        match fields.check_data(temporal_types.clone()) {
                            Ok(interal_types) => {
                                cycle_types.insert(name.clone(), interal_types);
                            }
                            Err(e) => {
                                contract_result.error = e;
                                return;
                            }
                        }
                    }

                    if let Err(e) = check_cycle(cycle_types) {
                        contract_result.error = e;
                        return;
                    };

                    state.custom_types = temporal_types;

                    state.version += 1;
                }
                ChangeTypes::Delete { names } => {
                    for name in names {
                        if state.custom_types.remove(&name).is_none() {
                            contract_result.error =
                                "The type to be deleted does not exist".to_owned();
                            return;
                        }
                    }

                    state.version += 1;
                }
            }
        }
        Events::ChangeProductionSystem (operation) => match operation {
            ChangeProductionSystem::Init { name, unit_process } => {
                if name == "" {
                    contract_result.error = "System name can not be empty".to_owned();
                    return;
                }

                if state.version != 0 {
                    contract_result.error = "Can not emit init event if version is != 0".to_owned();
                    return;
                }

                let mut unit_names = vec![];
                for unit in unit_process.clone() {
                    if unit.name.is_empty() {
                        contract_result.error = "Unit process name can not be empty".to_owned();
                        return;
                    }

                    unit_names.push(unit.name.clone());

                    if let Err(e) = unit.check_data(state.custom_types.clone()) {
                        contract_result.error = e;
                        return;
                    }
                }

                let hash_unit_name: HashSet<String> =
                    HashSet::from_iter(unit_names.iter().cloned());
                if hash_unit_name.len() != unit_names.len() {
                    contract_result.error = "Units procces has not uniques names".to_owned();
                    return;
                }

                state.version += 1;
                state.name = name;
                state.unit_process = unit_process;
            }
            ChangeProductionSystem::NewName { name } => {
                if state.version == 0 {
                    contract_result.error = "The first event must be Init event".to_owned();
                    return;
                }

                if name.is_empty() {
                    contract_result.error = "New name can not be empty".to_owned();
                    return;
                }

                state.version += 1;
                state.name = name;
            }
            ChangeProductionSystem::Modify {
                modification,
                add,
                delete,
            } => {
                if state.version == 0 {
                    contract_result.error = "The first event must be Init event".to_owned();
                    return;
                }

                // Incrementar la versión si se realizaron cambios
                if delete.is_empty() && modification.is_empty() && add.is_empty() {
                    contract_result.error = "To emit a modification event at least the modifications, add or delete vector cannot be empty".to_owned();
                    return;
                }

                // 1. Eliminar procesos
                for name in delete.clone() {
                    if let Some(pos) = state.unit_process.iter().position(|x| x.name == name) {
                        state.unit_process.remove(pos);
                    } else {
                        // Si un proceso a eliminar no existe, devolver
                        contract_result.error = format!("The process to be eliminated {} does not exist in the production system", name);
                        return;
                    }
                }

                // 2. Modificar procesos
                for (name, process) in modification.clone() {
                    if let Err(e) = process.check_data(state.custom_types.clone()) {
                        contract_result.error = e;
                        return;
                    };

                    if let Some(existing) = state.unit_process.iter_mut().find(|x| x.name == name) {
                        *existing = process;
                    } else {
                        // Si un proceso a modificar no existe, devolver
                        contract_result.error = format!("The process to be modificated {} does not exist in the production system", name);
                        return;
                    }
                }

                // 3. Añadir nuevos procesos
                for unit_process in add {
                    if let Err(e) = unit_process.check_data(state.custom_types.clone()) {
                        contract_result.error = e;
                        return;
                    }

                    state.unit_process.push(unit_process);
                }

                let unit_names = state
                    .unit_process
                    .iter()
                    .map(|x| x.name.clone())
                    .collect::<Vec<String>>();
                let hash_unit_name: HashSet<String> =
                    HashSet::from_iter(unit_names.iter().cloned());

                if hash_unit_name.len() != unit_names.len() {
                    contract_result.error = "Units procces has not uniques names".to_owned();
                    return;
                }

                state.version += 1;
            }
        },
        Events::RegisterData(data) => {
            if state.version == 0 {
                contract_result.error = "The first event must be Init event".to_owned();
                return;
            }

            let mut change = false;

            for unit_process in state.unit_process.iter_mut() {
                if unit_process.name == data.name {
                    if let Err(e) =
                        unit_process.register_data(data.clone(), state.custom_types.clone())
                    {
                        contract_result.error = e;
                        return;
                    };
                    change = true;
                    break;
                }
            }

            if !change {
                contract_result.error = format!("No processing unit found matching {}", data.name);
                return;
            }
        }
    }
    contract_result.success = true;
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, vec};

    use crate::{
        contract_logic, ChangeProductionSystem, ChangeTypes, Data, DynamicType, Events,
        Fields, ProductionSystem, Scopes, UnitData, UnitProcess,
    };
    use kore_contract_sdk as sdk;
    use serde_json::json;


    #[test]
    fn register_types_option_type_null() {
        let mut custom_type = Fields(HashMap::new());
        custom_type.0.insert("name".to_owned(), DynamicType::String);
        custom_type.0.insert(
            "value".to_owned(),
            DynamicType::Option(Box::new(DynamicType::String)),
        );

        let mut custom_type_2 = Fields(HashMap::new());
        custom_type_2.0.insert("data".to_owned(), DynamicType::Type("User".to_owned()));

        let mut types = HashMap::new();
        types.insert("User".to_owned(), custom_type);
        types.insert("UserData".to_owned(), custom_type_2);

        let unit_process_1 = UnitProcess {
            name: "grape treading".to_owned(),
            scope: vec![Scopes::Temporal(25)],
            main_outputs: vec![Data {
                name: "Farmer data".to_owned(),
                type_name: "UserData".to_owned(),
                content: json!({"data": {
                    "name": "Pepe",
                    "value": "Rock"
                }}),
            }],
            inputs: vec![],
            other_outputs: vec![],
        };

        let init_state = ProductionSystem {
            name: "wine process".to_owned(),
            version: 1,
            unit_process: vec![unit_process_1],
            custom_types: types,
        };

        let context = sdk::Context {
            initial_state: init_state.clone(),
            event: Events::RegisterData(UnitData {
                    name: "grape treading".to_owned(),
                    main_outputs: Some(vec![Data {
                        name: "Farmer data".to_owned(),
                        type_name: "UserData".to_owned(),
                        content: json!({"data": {
                            "name": "Andres"
                        }}),
                    }]),
                    inputs: None,
                    other_outputs: None,
                    scope: Some(vec![Scopes::Temporal(5)]),
                },
            ),
            is_owner: false,
        };

        let mut result = sdk::ContractResult::new(init_state);

        contract_logic(&context, &mut result);

        
        println!("{}", result.error);
        assert!(result.error.is_empty());
        
        let data = result.final_state.unit_process[0].main_outputs[0].clone();
        assert_eq!(data.name, "Farmer data");
        assert_eq!(data.type_name, "UserData");
        assert_eq!(data.content, json!({"data": {
            "name": "Andres"
        }}));
    }

    #[test]
    fn register_types_option_null() {
        let mut custom_type = Fields(HashMap::new());
        custom_type.0.insert("name".to_owned(), DynamicType::String);
        custom_type.0.insert(
            "value".to_owned(),
            DynamicType::Option(Box::new(DynamicType::String)),
        );

        let mut types = HashMap::new();
        types.insert("User".to_owned(), custom_type);

        let unit_process_1 = UnitProcess {
            name: "grape treading".to_owned(),
            scope: vec![Scopes::Temporal(25)],
            main_outputs: vec![Data {
                name: "Farmer data".to_owned(),
                type_name: "User".to_owned(),
                content: json!({"name": "pepe"}),
            }],
            inputs: vec![],
            other_outputs: vec![],
        };

        let init_state = ProductionSystem {
            name: "wine process".to_owned(),
            version: 1,
            unit_process: vec![unit_process_1],
            custom_types: types,
        };

        let context = sdk::Context {
            initial_state: init_state.clone(),
            event: Events::RegisterData(UnitData {
                    name: "grape treading".to_owned(),
                    main_outputs: Some(vec![Data {
                        name: "Farmer data".to_owned(),
                        type_name: "User".to_owned(),
                        content: json!({"name": "Andres"}),
                    }]),
                    inputs: None,
                    other_outputs: None,
                    scope: Some(vec![Scopes::Temporal(5)]),
                },
            ),
            is_owner: false,
        };

        let mut result = sdk::ContractResult::new(init_state);

        contract_logic(&context, &mut result);

        
        assert!(result.error.is_empty());
        let data = result.final_state.unit_process[0].main_outputs[0].clone();
        assert_eq!(data.name, "Farmer data");
        assert_eq!(data.type_name, "User");
        assert_eq!(data.content, json!({"name": "Andres"}));
    }

    #[test]
    fn register_types_option_all_null() {
        let mut custom_type = Fields(HashMap::new());
        custom_type.0.insert("name".to_owned(), DynamicType::Option(Box::new(DynamicType::String)));
        custom_type.0.insert(
            "value".to_owned(),
            DynamicType::Option(Box::new(DynamicType::String)),
        );

        let mut types = HashMap::new();
        types.insert("User".to_owned(), custom_type);

        let unit_process_1 = UnitProcess {
            name: "grape treading".to_owned(),
            scope: vec![Scopes::Temporal(25)],
            main_outputs: vec![Data {
                name: "Farmer data".to_owned(),
                type_name: "User".to_owned(),
                content: json!({"name": "pepe"}),
            }],
            inputs: vec![],
            other_outputs: vec![],
        };

        let init_state = ProductionSystem {
            name: "wine process".to_owned(),
            version: 1,
            unit_process: vec![unit_process_1],
            custom_types: types,
        };

        let context = sdk::Context {
            initial_state: init_state.clone(),
            event: Events::RegisterData(UnitData {
                    name: "grape treading".to_owned(),
                    main_outputs: Some(vec![Data {
                        name: "Farmer data".to_owned(),
                        type_name: "User".to_owned(),
                        content: json!({}),
                    }]),
                    inputs: None,
                    other_outputs: None,
                    scope: Some(vec![Scopes::Temporal(5)]),
                },
            ),
            is_owner: false,
        };

        let mut result = sdk::ContractResult::new(init_state);

        contract_logic(&context, &mut result);

        
        assert!(result.error.is_empty());
        let data = result.final_state.unit_process[0].main_outputs[0].clone();
            assert_eq!(data.name, "Farmer data");
            assert_eq!(data.type_name, "User");
            assert_eq!(data.content, json!({}));
    }

    #[test]
    fn register_types_option() {
        let mut custom_type = Fields(HashMap::new());
        custom_type.0.insert("name".to_owned(), DynamicType::String);
        custom_type.0.insert(
            "value".to_owned(),
            DynamicType::Option(Box::new(DynamicType::String)),
        );

        let mut types = HashMap::new();
        types.insert("User".to_owned(), custom_type);

        let unit_process_1 = UnitProcess {
            name: "grape treading".to_owned(),
            scope: vec![Scopes::Temporal(25)],
            main_outputs: vec![Data {
                name: "Farmer data".to_owned(),
                type_name: "User".to_owned(),
                content: json!({"name": "pepe"}),
            }],
            inputs: vec![],
            other_outputs: vec![],
        };

        let init_state = ProductionSystem {
            name: "wine process".to_owned(),
            version: 1,
            unit_process: vec![unit_process_1],
            custom_types: types,
        };

        let context = sdk::Context {
            initial_state: init_state.clone(),
            event: Events::RegisterData(UnitData {
                    name: "grape treading".to_owned(),
                    main_outputs: Some(vec![Data {
                        name: "Farmer data".to_owned(),
                        type_name: "User".to_owned(),
                        content: json!({"name": "Andres", "value": "val"}),
                    }]),
                    inputs: None,
                    other_outputs: None,
                    scope: Some(vec![Scopes::Temporal(5)]),
                },
            ),
            is_owner: false,
        };

        let mut result = sdk::ContractResult::new(init_state);

        contract_logic(&context, &mut result);

        assert!(result.error.is_empty());
        let data = result.final_state.unit_process[0].main_outputs[0].clone();
        assert_eq!(data.name, "Farmer data");
        assert_eq!(data.type_name, "User");
        assert_eq!(data.content, json!({"name": "Andres", "value": "val"}));
    }

    #[test]
    fn register_types_option_2() {
        let mut custom_type = Fields(HashMap::new());
        custom_type.0.insert("name".to_owned(), DynamicType::String);
        custom_type.0.insert(
            "value".to_owned(),
            DynamicType::Option(Box::new(DynamicType::String)),
        );

        let mut types = HashMap::new();
        types.insert("User".to_owned(), custom_type);

        let unit_process_1 = UnitProcess {
            name: "grape treading".to_owned(),
            scope: vec![Scopes::Temporal(25)],
            main_outputs: vec![Data {
                name: "Farmer data".to_owned(),
                type_name: "User".to_owned(),
                content: json!({"name": "pepe", "value": "lav"}),
            }],
            inputs: vec![],
            other_outputs: vec![],
        };

        let init_state = ProductionSystem {
            name: "wine process".to_owned(),
            version: 1,
            unit_process: vec![unit_process_1],
            custom_types: types,
        };

        let context = sdk::Context {
            initial_state: init_state.clone(),
            event: Events::RegisterData(UnitData {
                    name: "grape treading".to_owned(),
                    main_outputs: Some(vec![Data {
                        name: "Farmer data".to_owned(),
                        type_name: "User".to_owned(),
                        content: json!({"name": "Andres"}),
                    }]),
                    inputs: None,
                    other_outputs: None,
                    scope: Some(vec![Scopes::Temporal(5)]),
                },
            ),
            is_owner: false,
        };

        let mut result = sdk::ContractResult::new(init_state);

        contract_logic(&context, &mut result);

        
        assert!(result.error.is_empty());
        let data = result.final_state.unit_process[0].main_outputs[0].clone();
            assert_eq!(data.name, "Farmer data");
            assert_eq!(data.type_name, "User");
            assert_eq!(data.content, json!({"name": "Andres"}));
    }

    #[test]
    fn register_types_single() {
        let unit_process_1 = UnitProcess {
            name: "grape treading".to_owned(),
            scope: vec![Scopes::Temporal(25)],
            main_outputs: vec![Data {
                name: "Farmer data".to_owned(),
                type_name: "f64".to_owned(),
                content: json!(0.0),
            }],
            inputs: vec![],
            other_outputs: vec![],
        };

        let init_state = ProductionSystem {
            name: "wine process".to_owned(),
            version: 1,
            unit_process: vec![unit_process_1],
            custom_types: HashMap::new(),
        };

        let context = sdk::Context {
            initial_state: init_state.clone(),
            event: Events::RegisterData(UnitData {
                    name: "grape treading".to_owned(),
                    main_outputs: Some(vec![Data {
                        name: "Farmer data".to_owned(),
                        type_name: "f64".to_owned(),
                        content: json!(15.5),
                    }]),
                    inputs: None,
                    other_outputs: None,
                    scope: Some(vec![Scopes::Temporal(5)]),
                },
            ),
            is_owner: false,
        };

        let mut result = sdk::ContractResult::new(init_state);

        contract_logic(&context, &mut result);

        
        assert!(result.error.is_empty());

        let data = result.final_state.unit_process[0].main_outputs[0].clone();
            assert_eq!(data.name, "Farmer data");
            assert_eq!(data.type_name, "f64");
            assert_eq!(data.content, json!(15.5));
    }

    #[test]
    fn register_types_basic() {
        let mut custom_type = Fields(HashMap::new());
        custom_type.0.insert("name".to_owned(), DynamicType::String);
        custom_type.0.insert("value".to_owned(), DynamicType::f64);

        let mut types = HashMap::new();
        types.insert("User".to_owned(), custom_type);

        let unit_process_1 = UnitProcess {
            name: "grape treading".to_owned(),
            scope: vec![Scopes::Temporal(25)],
            main_outputs: vec![Data {
                name: "Farmer data".to_owned(),
                type_name: "User".to_owned(),
                content: json!({"name": "pepe", "value": 0.5}),
            }],
            inputs: vec![],
            other_outputs: vec![],
        };

        let init_state = ProductionSystem {
            name: "wine process".to_owned(),
            version: 1,
            unit_process: vec![unit_process_1],
            custom_types: types,
        };

        let context = sdk::Context {
            initial_state: init_state.clone(),
            event: Events::RegisterData(UnitData {
                    name: "grape treading".to_owned(),
                    main_outputs: Some(vec![Data {
                        name: "Farmer data".to_owned(),
                        type_name: "User".to_owned(),
                        content: json!({"name": "Andres", "value": 5.1}),
                    }]),
                    inputs: None,
                    other_outputs: None,
                    scope: Some(vec![Scopes::Temporal(5)]),
                },
            ),
            is_owner: false,
        };

        let mut result = sdk::ContractResult::new(init_state);

        contract_logic(&context, &mut result);

        assert!(result.error.is_empty());

        let data = result.final_state.unit_process[0].main_outputs[0].clone();
            assert_eq!(data.name, "Farmer data");
            assert_eq!(data.type_name, "User");
            assert_eq!(data.content, json!({"name": "Andres", "value": 5.1}));
    }

    #[test]
    fn register_types_vec() {
        let mut custom_type = Fields(HashMap::new());
        custom_type.0.insert("name".to_owned(), DynamicType::String);
        custom_type.0.insert(
            "values".to_owned(),
            DynamicType::Vec(Box::new(DynamicType::i64)),
        );

        let mut types = HashMap::new();
        types.insert("User".to_owned(), custom_type);

        let unit_process_1 = UnitProcess {
            name: "grape treading".to_owned(),
            scope: vec![Scopes::Temporal(25)],
            main_outputs: vec![Data {
                name: "Farmer data".to_owned(),
                type_name: "User".to_owned(),
                content: json!({"name": "pepe", "values": []}),
            }],
            inputs: vec![],
            other_outputs: vec![],
        };

        let init_state = ProductionSystem {
            name: "wine process".to_owned(),
            version: 1,
            unit_process: vec![unit_process_1],
            custom_types: types,
        };

        let context = sdk::Context {
            initial_state: init_state.clone(),
            event: Events::RegisterData(UnitData {
                    name: "grape treading".to_owned(),
                    main_outputs: Some(vec![Data {
                        name: "Farmer data".to_owned(),
                        type_name: "User".to_owned(),
                        content: json!({"name": "Andres", "values": [12, 33, 551]}),
                    }]),
                    inputs: None,
                    other_outputs: None,
                    scope: Some(vec![Scopes::Temporal(5)]),
                },
            ),
            is_owner: false,
        };

        let mut result = sdk::ContractResult::new(init_state);

        contract_logic(&context, &mut result);

        assert!(result.error.is_empty());
        let data = result.final_state.unit_process[0].main_outputs[0].clone();
            assert_eq!(data.name, "Farmer data");
            assert_eq!(data.type_name, "User");
            assert_eq!(
                data.content,
                json!({"name": "Andres", "values": [12, 33, 551]})
            );
        
    }

    #[test]
    fn register_types_enum() {
        let mut object = HashMap::new();
        object.insert("animal".to_owned(), DynamicType::String);
        object.insert("age".to_owned(), DynamicType::u64);

        let mut custom_type = Fields(HashMap::new());
        custom_type.0.insert("name".to_owned(), DynamicType::String);
        custom_type
            .0
            .insert("values".to_owned(), DynamicType::Enum(Fields(object)));

        let mut types = HashMap::new();
        types.insert("User".to_owned(), custom_type);

        let unit_process_1 = UnitProcess {
            name: "grape treading".to_owned(),
            scope: vec![Scopes::Temporal(25)],
            main_outputs: vec![Data {
                name: "Farmer data".to_owned(),
                type_name: "User".to_owned(),
                content: json!({"name": "", "values": {}}),
            }],
            inputs: vec![],
            other_outputs: vec![],
        };

        let init_state = ProductionSystem {
            name: "wine process".to_owned(),
            version: 1,
            unit_process: vec![unit_process_1],
            custom_types: types,
        };

        let context = sdk::Context {
            initial_state: init_state.clone(),
            event: Events::RegisterData(UnitData {
                    name: "grape treading".to_owned(),
                    main_outputs: Some(vec![Data {
                        name: "Farmer data".to_owned(),
                        type_name: "User".to_owned(),
                        content: json!({"name": "Andres", "values": {"age": 21}}),
                    }]),
                    inputs: None,
                    other_outputs: None,
                    scope: Some(vec![Scopes::Temporal(5)]),
                },
            ),
            is_owner: false,
        };

        let mut result = sdk::ContractResult::new(init_state);

        contract_logic(&context, &mut result);

        assert!(result.error.is_empty());
        let data = result.final_state.unit_process[0].main_outputs[0].clone();
            assert_eq!(data.name, "Farmer data");
            assert_eq!(data.type_name, "User");
            assert_eq!(
                data.content,
                json!({"name": "Andres", "values": {"age": 21}})
            );
        
    }

    #[test]
    fn register_types_type() {
        let mut custom_type = Fields(HashMap::new());
        custom_type.0.insert("name".to_owned(), DynamicType::String);
        custom_type.0.insert("value".to_owned(), DynamicType::f64);

        let mut types = HashMap::new();
        types.insert("User".to_owned(), custom_type);

        let unit_process_1 = UnitProcess {
            name: "grape treading".to_owned(),
            scope: vec![Scopes::Temporal(25)],
            main_outputs: vec![Data {
                name: "Farmer data".to_owned(),
                type_name: "User".to_owned(),
                content: json!({"name": "pepe", "value": 0.5}),
            }],
            inputs: vec![],
            other_outputs: vec![],
        };

        let init_state = ProductionSystem {
            name: "wine process".to_owned(),
            version: 1,
            unit_process: vec![unit_process_1],
            custom_types: types,
        };

        let mut custom_type = Fields(HashMap::new());
        custom_type
            .0
            .insert("user_data".to_owned(), DynamicType::Type("User".to_owned()));
        custom_type
            .0
            .insert("value".to_owned(), DynamicType::String);
        let context = sdk::Context {
            initial_state: init_state.clone(),
            event: Events::ChangeTypes(ChangeTypes::Add {
                    types: vec![("Another User".to_owned(), custom_type)],
                },
            ),
            is_owner: false,
        };

        let mut result = sdk::ContractResult::new(init_state.clone());

        contract_logic(&context, &mut result);

        assert!(result.error.is_empty());
        assert_eq!(result.final_state.custom_types.len(), 2);
    }

    #[test]
    fn register_types_type_cycle() {
        let mut custom_type = Fields(HashMap::new());
        custom_type.0.insert("name".to_owned(), DynamicType::String);
        custom_type.0.insert(
            "value".to_owned(),
            DynamicType::Type("Another User".to_owned()),
        );

        let mut types = HashMap::new();
        types.insert("User".to_owned(), custom_type);

        let unit_process_1 = UnitProcess {
            name: "grape treading".to_owned(),
            scope: vec![Scopes::Temporal(25)],
            main_outputs: vec![],
            inputs: vec![],
            other_outputs: vec![],
        };

        let init_state = ProductionSystem {
            name: "wine process".to_owned(),
            version: 1,
            unit_process: vec![unit_process_1],
            custom_types: types,
        };

        let mut custom_type = Fields(HashMap::new());
        custom_type.0.insert(
            "user_data".to_owned(),
            DynamicType::Type("Fake User".to_owned()),
        );
        custom_type
            .0
            .insert("value".to_owned(), DynamicType::String);

        let mut custom_type_2 = Fields(HashMap::new());
        custom_type_2
            .0
            .insert("user_data".to_owned(), DynamicType::Type("User".to_owned()));
        custom_type_2
            .0
            .insert("value".to_owned(), DynamicType::String);

        let context = sdk::Context {
            initial_state: init_state.clone(),
            event: Events::ChangeTypes(ChangeTypes::Add {
                    types: vec![
                        ("Another User".to_owned(), custom_type),
                        ("Fake User".to_owned(), custom_type_2),
                    ],
                },
            ),
            is_owner: false,
        };

        let mut result = sdk::ContractResult::new(init_state.clone());

        contract_logic(&context, &mut result);

        assert!(!result.error.is_empty());
        assert!(!result.success);
    }

    #[test]
    fn register_types_type_not_cycle() {
        let mut custom_type = Fields(HashMap::new());
        custom_type.0.insert("name".to_owned(), DynamicType::String);
        custom_type
            .0
            .insert("value".to_owned(), DynamicType::String);

        let mut types = HashMap::new();
        types.insert("User".to_owned(), custom_type);

        let unit_process_1 = UnitProcess {
            name: "grape treading".to_owned(),
            scope: vec![Scopes::Temporal(25)],
            main_outputs: vec![],
            inputs: vec![],
            other_outputs: vec![],
        };

        let init_state = ProductionSystem {
            name: "wine process".to_owned(),
            version: 1,
            unit_process: vec![unit_process_1],
            custom_types: types,
        };

        let mut custom_type = Fields(HashMap::new());
        custom_type
            .0
            .insert("user1".to_owned(), DynamicType::Type("User".to_owned()));
        custom_type
            .0
            .insert("user2".to_owned(), DynamicType::Type("User".to_owned()));

        let context = sdk::Context {
            initial_state: init_state.clone(),
            event: Events::ChangeTypes(ChangeTypes::Add {
                    types: vec![("Another User".to_owned(), custom_type)],
                },
            ),
            is_owner: false,
        };

        let mut result = sdk::ContractResult::new(init_state.clone());

        contract_logic(&context, &mut result);

        assert!(result.error.is_empty());
        assert!(result.success);
    }

    #[test]
    fn register_types_value() {
        // Registramos
        let mut custom_type_user = Fields(HashMap::new());
        custom_type_user
            .0
            .insert("name".to_owned(), DynamicType::String);
        custom_type_user
            .0
            .insert("value".to_owned(), DynamicType::f64);

        let mut types = HashMap::new();
        types.insert("User".to_owned(), custom_type_user);

        let mut custom_type_another_user = Fields(HashMap::new());
        custom_type_another_user
            .0
            .insert("user".to_owned(), DynamicType::Type("User".to_owned()));
        custom_type_another_user
            .0
            .insert("value".to_owned(), DynamicType::f64);

        types.insert("Another User".to_owned(), custom_type_another_user);

        let unit_process_1 = UnitProcess {
            name: "grape treading".to_owned(),
            scope: vec![Scopes::Temporal(25)],
            main_outputs: vec![Data {
                name: "Farmer data".to_owned(),
                type_name: "Another User".to_owned(),
                content: json!({"user": {"name": "", "value": 0.0}, "value": 0.0}),
            }],
            inputs: vec![],
            other_outputs: vec![],
        };

        let init_state = ProductionSystem {
            name: "wine process".to_owned(),
            version: 1,
            unit_process: vec![unit_process_1],
            custom_types: types,
        };

        let context = sdk::Context {
            initial_state: init_state.clone(),
            event: Events::RegisterData(UnitData {
                    name: "grape treading".to_owned(),
                    main_outputs: Some(vec![Data {
                        name: "Farmer data".to_owned(),
                        type_name: "Another User".to_owned(),
                        content: json!({"user": { "name": "Alberto", "value": 5.5}, "value": 0.6}),
                    }]),
                    inputs: None,
                    other_outputs: None,
                    scope: Some(vec![Scopes::Temporal(5)]),
                },
            ),
            is_owner: false,
        };

        let mut result = sdk::ContractResult::new(init_state);

        contract_logic(&context, &mut result);

        assert!(result.error.is_empty());

        let data = result.final_state.unit_process[0].main_outputs[0].clone();
        assert_eq!(data.name, "Farmer data");
        assert_eq!(data.type_name, "Another User");
        assert_eq!(
            data.content,
            json!({"user": { "name": "Alberto", "value": 5.5}, "value": 0.6})
        );

    }

    #[test]
    fn change_operation_name() {
        let init_state = ProductionSystem {
            name: "wine process".to_owned(),
            version: 1,
            unit_process: vec![],
            custom_types: HashMap::new(),
        };

        let context = sdk::Context {
            initial_state: init_state.clone(),
            event: Events::ChangeProductionSystem(ChangeProductionSystem::NewName {
                    name: "wine process 2".to_owned(),
                },
            ),
            is_owner: false,
        };

        let mut result = sdk::ContractResult::new(init_state);
        contract_logic(&context, &mut result);

        assert_eq!(result.final_state.version, 2);
        assert_eq!(result.final_state.name, "wine process 2");
        assert!(result.success);
    }

    #[test]
    fn test_change_operation_init() {
        let init_state = ProductionSystem {
            name: "".to_owned(),
            version: 0,
            unit_process: vec![],
            custom_types: HashMap::new(),
        };

        let context = sdk::Context {
            initial_state: init_state.clone(),
            event: Events::ChangeProductionSystem(ChangeProductionSystem::Init {
                    name: "wine process".to_owned(),
                    unit_process: vec![],
                },
            ),
            is_owner: false,
        };

        let mut result = sdk::ContractResult::new(init_state);
        contract_logic(&context, &mut result);

        assert_eq!(result.final_state.name, "wine process");
        assert_eq!(result.final_state.version, 1);
        assert_eq!(result.final_state.unit_process.len(), 0);
        assert!(result.success);
    }
/*
// Define the elements of the production system.
#[derive(Serialize, Deserialize, Clone, Debug)]
struct Element {
    // Caudal de aceite
        pub name: String,
    // Waterflow
        pub element_type: String,
    // Agua
        pub category: String,
    // Caudal volumetrico
        pub element_name: String,
    // m^3/h
        pub unit: String,
    // Tipo de evento
        pub manual_event: bool,
    // Son x botellas al día, es un valor fijo
    // Una medida del caudal tomada cada x días
    pub absolute: bool,
    // El que se mida
    pub measure: Vec<(f64, Vec<Scopes>)>,
}
*/
    
    #[test]
    fn test_change_operation_new() {
        let mut custom_type_element = Fields(HashMap::new());
        custom_type_element.0.insert("element_type".to_owned(), DynamicType::String);
        custom_type_element.0.insert("category".to_owned(), DynamicType::String);
        custom_type_element.0.insert("element_name".to_owned(), DynamicType::String);
        custom_type_element.0.insert("unit".to_owned(), DynamicType::String);
        custom_type_element.0.insert("manual_event".to_owned(), DynamicType::bool);
        custom_type_element.0.insert("absolute".to_owned(), DynamicType::bool);
        custom_type_element.0.insert("measure".to_owned(), DynamicType::Vec(Box::new(DynamicType::Type("MeasureType".to_owned()))));
        let mut custom_type_scopes = Fields(HashMap::new());
        custom_type_scopes.0.insert("Temporal".to_owned(), DynamicType::u64);
        custom_type_scopes.0.insert("TimeZone".to_owned(), DynamicType::String);
        custom_type_scopes.0.insert("Tag".to_owned(), DynamicType::String);
        let mut custom_type_measure = Fields(HashMap::new());
        custom_type_measure.0.insert("value".to_owned(), DynamicType::f64);
        custom_type_measure.0.insert("scopes".to_owned(), DynamicType::Vec(Box::new(DynamicType::Enum(custom_type_scopes))));

        let mut types = HashMap::new();
        types.insert("Element".to_owned(), custom_type_element);
        types.insert("MeasureType".to_owned(), custom_type_measure);

        let init_state = ProductionSystem {
            name: "wine process".to_owned(),
            version: 1,
            unit_process: vec![],
            custom_types: types,
        };

        let unit_process_1 = UnitProcess {
            name: "grape treading".to_owned(),
            scope: vec![Scopes::Temporal(25)],
            main_outputs: vec![Data {
                name: "Caudal volumetrico del aceite".to_owned(),
                type_name: "Element".to_owned(),
                content: json!({
                    "element_name": "Caudal volumetrico".to_owned(),
                    "element_type": "WaterFlow".to_owned(),
                    "category": "Caudal".to_owned(),
                    "unit": "m^3/h".to_owned(),
                    "manual_event": false,
                    "absolute": false,
                    "measure": []
                })
            }],
            inputs: vec![
                Data {
                    name: "PH del agua".to_owned(),
                    type_name: "Element".to_owned(),
                    content: json!({
                        "element_name": "PH".to_owned(),
                        "element_type": "WaterFlow".to_owned(),
                        "category": "Water".to_owned(),
                        "unit": "pH".to_owned(),
                        "absolute": false,
                        "manual_event": false,
                        "measure": [],
                    })
                }],
            other_outputs: vec![
                Data {
                    name: "Caudal volumetrico del aceite2".to_owned(),
                    type_name: "Element".to_owned(),
                    content: json!({
                        "absolute": false,
                        "element_name": "Caudal volumetrico".to_owned(),
                        "element_type": "WaterFlow".to_owned(),
                        "category": "Caudal".to_owned(),
                        "unit": "m^3/h".to_owned(),
                        "manual_event": false,
                        "measure": [],
                    })
                }],
        };

        let context = sdk::Context {
            initial_state: init_state.clone(),
            event: Events::ChangeProductionSystem(ChangeProductionSystem::Modify {
                    delete: vec![],
                    modification: vec![],
                    add: vec![unit_process_1],
                },
            ),
            is_owner: false,
        };

        let mut result = sdk::ContractResult::new(init_state);

        contract_logic(&context, &mut result);

        println!("{}", result.error);
        assert!(result.error.is_empty());

        assert_eq!(result.final_state.version, 2);
        assert_eq!(result.final_state.unit_process.len(), 1);
        assert_eq!(result.final_state.unit_process[0].name, "grape treading");
        assert_eq!(result.final_state.unit_process[0].other_outputs.len(), 1);
        assert_eq!(result.final_state.unit_process[0].inputs.len(), 1);

        let element = result.final_state.unit_process[0].inputs[0].clone();
        assert_eq!(element.name, "PH del agua");
        assert_eq!(element.content, json!({
            "element_name": "PH".to_owned(),
            "element_type": "WaterFlow".to_owned(),
            "category": "Water".to_owned(),
            "unit": "pH".to_owned(),
            "absolute": false,
            "manual_event": false,
            "measure": [],
        }));
        

        let element= result.final_state.unit_process[0].main_outputs[0].clone();
        assert_eq!(element.name, "Caudal volumetrico del aceite");
        assert_eq!(element.content, json!({
            "element_name": "Caudal volumetrico".to_owned(),
            "element_type": "WaterFlow".to_owned(),
            "category": "Caudal".to_owned(),
            "manual_event": false,
            "unit": "m^3/h".to_owned(),
            "absolute": false,
            "measure": []
        }));

        let element = result.final_state.unit_process[0].other_outputs[0].clone();
        assert_eq!(element.name, "Caudal volumetrico del aceite2");
        assert_eq!(element.content, json!({
            "absolute": false,
            "element_name": "Caudal volumetrico".to_owned(),
            "element_type": "WaterFlow".to_owned(),
            "category": "Caudal".to_owned(),
            "unit": "m^3/h".to_owned(),
            "manual_event": false,
            "measure": [],
        }));

        if let Scopes::Temporal(temporal) = result.final_state.unit_process[0].scope[0] {
            assert_eq!(temporal, 25);
        } else {
            panic!("Invalid Scope")
        }

        // fixed and manual_event
        assert!(result.success);
    }

    
    #[test]
    fn test_change_operation_add() {
        let mut custom_type_element = Fields(HashMap::new());
        custom_type_element.0.insert("element_type".to_owned(), DynamicType::String);
        custom_type_element.0.insert("category".to_owned(), DynamicType::String);
        custom_type_element.0.insert("element_name".to_owned(), DynamicType::String);
        custom_type_element.0.insert("unit".to_owned(), DynamicType::String);
        custom_type_element.0.insert("manual_event".to_owned(), DynamicType::bool);
        custom_type_element.0.insert("absolute".to_owned(), DynamicType::bool);
        custom_type_element.0.insert("measure".to_owned(), DynamicType::Vec(Box::new(DynamicType::Type("MeasureType".to_owned()))));
        let mut custom_type_scopes = Fields(HashMap::new());
        custom_type_scopes.0.insert("Temporal".to_owned(), DynamicType::u64);
        custom_type_scopes.0.insert("TimeZone".to_owned(), DynamicType::String);
        custom_type_scopes.0.insert("Tag".to_owned(), DynamicType::String);
        let mut custom_type_measure = Fields(HashMap::new());
        custom_type_measure.0.insert("value".to_owned(), DynamicType::f64);
        custom_type_measure.0.insert("scopes".to_owned(), DynamicType::Vec(Box::new(DynamicType::Enum(custom_type_scopes))));

        let mut types = HashMap::new();
        types.insert("Element".to_owned(), custom_type_element);
        types.insert("MeasureType".to_owned(), custom_type_measure);

        let unit_process_1 = UnitProcess {
            name: "grape treading".to_owned(),
            main_outputs: vec![Data {
                name: "Caudal volumetrico del aceite".to_owned(),
                type_name: "Element".to_owned(),
                content: json!({
                    "element_name": "Caudal volumetrico".to_owned(),
                    "element_type": "WaterFlow".to_owned(),
                    "category": "Caudal".to_owned(),
                    "unit": "m^3/h".to_owned(),
                    "manual_event": false,
                    "absolute": false,
                    "measure": []
                })
            }],
            inputs: vec![Data {
                name: "PH del agua".to_owned(),
                type_name: "Element".to_owned(),
                content: json!({
                    "element_name": "PH".to_owned(),
                    "element_type": "WaterFlow".to_owned(),
                    "category": "Water".to_owned(),
                    "unit": "pH".to_owned(),
                    "absolute": false,
                    "manual_event": false,
                    "measure": [],
                })
            }],
            other_outputs: vec![],
            scope: vec![Scopes::Temporal(25)],
        };

        let init_state = ProductionSystem {
            name: "wine process".to_owned(),
            version: 1,
            unit_process: vec![unit_process_1],
            custom_types: types,
        };

        let unit_process_2 = UnitProcess {
            name: "bottling of wine".to_owned(),
            main_outputs: vec![
                Data {
                    name: "Caudal volumetrico2 del aceite".to_owned(),
                    type_name: "Element".to_owned(),
                    content: json!({
                        "element_name": "Caudal volumetrico2".to_owned(),
                        "element_type": "WaterFlow2".to_owned(),
                        "category": "Caudal2".to_owned(),
                        "unit": "m^3/h2".to_owned(),
                        "measure": [],
                        "absolute": false,
                        "manual_event": false,
                    })
                }],
            inputs: vec![
                Data {
                    name: "PH2 del agua".to_owned(),
                    type_name: "Element".to_owned(),
                    content: json!({
                        "element_type": "WaterFlow2".to_owned(),
                        "category": "Water2".to_owned(),
                        "element_name": "PH2".to_owned(),
                        "unit": "pH2".to_owned(),
                        "measure": [],
                        "manual_event": false,
                        "absolute": false,
                    })
                }],
            other_outputs: vec![],
            scope: vec![Scopes::Temporal(33)],
        };
        let unit_process_3 = UnitProcess {
            name: "bottling of winee".to_owned(),
            main_outputs: vec![
                Data {
                    name: "Caudal volumetrico3 del aceite".to_owned(),
                    type_name: "Element".to_owned(),
                    content: json!({
                        "element_name": "Caudal volumetrico2".to_owned(),
                        "element_type": "WaterFlow2".to_owned(),
                        "category": "Caudal3".to_owned(),
                        "unit": "m^3/h2".to_owned(),
                        "measure": [],
                        "absolute": false,
                        "manual_event": false,
                    })
                }],
            inputs: vec![
                Data {
                    name: "PH3 del agua".to_owned(),
                    type_name: "Element".to_owned(),
                    content: json!({
                        "element_type": "WaterFlow2".to_owned(),
                        "category": "Water2".to_owned(),
                        "element_name": "PH3".to_owned(),
                        "unit": "pH2".to_owned(),
                        "measure": [{
                            "value": 0.5,
                            "scopes": [
                                {
                                    "Temporal": 5555
                                },
                                {
                                    "Tag": "IA"
                                },
                                {
                                    "TimeZone": "UTC-0"
                                },
                            ]
                        }],
                        "manual_event": false,
                        "absolute": false,
                    })
                }],
            other_outputs: vec![],
            scope: vec![Scopes::Temporal(33)],
        };

        let context = sdk::Context {
            initial_state: init_state.clone(),
            event: Events::ChangeProductionSystem(ChangeProductionSystem::Modify {
                    delete: vec![],
                    modification: vec![],
                    add: vec![unit_process_2, unit_process_3],
                },
            ),
            is_owner: false,
        };

        let mut result = sdk::ContractResult::new(init_state);
        contract_logic(&context, &mut result);

        assert!(result.error.is_empty());

        assert_eq!(result.final_state.version, 2);
        assert_eq!(result.final_state.unit_process.len(), 3);
        assert_eq!(result.final_state.unit_process[0].name, "grape treading");
        assert_eq!(result.final_state.unit_process[0].other_outputs.len(), 0);

        let element = result.final_state.unit_process[0].inputs[0].clone();
        assert_eq!(element.name, "PH del agua");
        assert_eq!(element.content, json!({
            "element_name": "PH".to_owned(),
            "element_type": "WaterFlow".to_owned(),
            "category": "Water".to_owned(),
            "unit": "pH".to_owned(),
            "absolute": false,
            "manual_event": false,
            "measure": [],
        }));

        let element = result.final_state.unit_process[1].inputs[0].clone();
        assert_eq!(element.name, "PH2 del agua");
        assert_eq!(element.content, json!({
            "element_type": "WaterFlow2".to_owned(),
            "category": "Water2".to_owned(),
            "element_name": "PH2".to_owned(),
            "unit": "pH2".to_owned(),
            "measure": [],
            "manual_event": false,
            "absolute": false,
        }));

        let element = result.final_state.unit_process[2].inputs[0].clone();
        assert_eq!(element.name, "PH3 del agua");
        assert_eq!(element.content, json!({
            "element_type": "WaterFlow2".to_owned(),
            "category": "Water2".to_owned(),
            "element_name": "PH3".to_owned(),
            "unit": "pH2".to_owned(),
            "measure": [{
                "value": 0.5,
                "scopes": [
                    {
                        "Temporal": 5555
                    },
                    {
                        "Tag": "IA"
                    },
                    {
                        "TimeZone": "UTC-0"
                    },
                ]
            }],
            "manual_event": false,
            "absolute": false,
        }));

        let element = result.final_state.unit_process[0].main_outputs[0].clone();
        assert_eq!(element.name, "Caudal volumetrico del aceite");
        assert_eq!(element.content, json!({
            "element_name": "Caudal volumetrico".to_owned(),
            "element_type": "WaterFlow".to_owned(),
            "category": "Caudal".to_owned(),
            "unit": "m^3/h".to_owned(),
            "manual_event": false,
            "absolute": false,
            "measure": []
        }));

        let element = result.final_state.unit_process[1].main_outputs[0].clone();
        assert_eq!(element.name, "Caudal volumetrico2 del aceite");
        assert_eq!(element.content, json!({
            "element_name": "Caudal volumetrico2".to_owned(),
            "element_type": "WaterFlow2".to_owned(),
            "category": "Caudal2".to_owned(),
            "unit": "m^3/h2".to_owned(),
            "measure": [],
            "absolute": false,
            "manual_event": false,
        }));

        let element = result.final_state.unit_process[2].main_outputs[0].clone();
        assert_eq!(element.name, "Caudal volumetrico3 del aceite");
        assert_eq!(element.content, json!({
            "element_name": "Caudal volumetrico2".to_owned(),
            "element_type": "WaterFlow2".to_owned(),
            "category": "Caudal3".to_owned(),
            "unit": "m^3/h2".to_owned(),
            "measure": [],
            "absolute": false,
            "manual_event": false,
        }));

        if let Scopes::Temporal(temporal) = result.final_state.unit_process[0].scope[0] {
            assert_eq!(temporal, 25);
        } else {
            panic!("Invalid Scope")
        }
        assert_eq!(result.final_state.unit_process[1].name, "bottling of wine");
        assert_eq!(result.final_state.unit_process[1].other_outputs.len(), 0);

        assert_eq!(result.final_state.unit_process[2].name, "bottling of winee");
        assert_eq!(result.final_state.unit_process[2].other_outputs.len(), 0);

        if let Scopes::Temporal(temporal) = result.final_state.unit_process[1].scope[0] {
            assert_eq!(temporal, 33);
        } else {
            panic!("Invalid Scope")
        }

        assert!(result.success);
    }

    
    #[test]
    fn test_change_operation_delete() {
        let mut custom_type_element = Fields(HashMap::new());
        custom_type_element.0.insert("element_type".to_owned(), DynamicType::String);
        custom_type_element.0.insert("category".to_owned(), DynamicType::String);
        custom_type_element.0.insert("element_name".to_owned(), DynamicType::String);
        custom_type_element.0.insert("unit".to_owned(), DynamicType::String);
        custom_type_element.0.insert("manual_event".to_owned(), DynamicType::bool);
        custom_type_element.0.insert("absolute".to_owned(), DynamicType::bool);
        custom_type_element.0.insert("measure".to_owned(), DynamicType::Vec(Box::new(DynamicType::Type("MeasureType".to_owned()))));
        let mut custom_type_scopes = Fields(HashMap::new());
        custom_type_scopes.0.insert("Temporal".to_owned(), DynamicType::u64);
        custom_type_scopes.0.insert("TimeZone".to_owned(), DynamicType::String);
        custom_type_scopes.0.insert("Tag".to_owned(), DynamicType::String);
        let mut custom_type_measure = Fields(HashMap::new());
        custom_type_measure.0.insert("value".to_owned(), DynamicType::f64);
        custom_type_measure.0.insert("scopes".to_owned(), DynamicType::Vec(Box::new(DynamicType::Enum(custom_type_scopes))));

        let mut types = HashMap::new();
        types.insert("Element".to_owned(), custom_type_element);
        types.insert("MeasureType".to_owned(), custom_type_measure);

        let unit_process_1 = UnitProcess {
            name: "grape treading".to_owned(),
            main_outputs: vec![Data {
                name: "Caudal volumetrico del aceite".to_owned(),
                type_name: "Element".to_owned(),
                content: json!({
                    "element_name": "Caudal volumetrico".to_owned(),
                    "element_type": "WaterFlow".to_owned(),
                    "category": "Caudal".to_owned(),
                    "unit": "m^3/h".to_owned(),
                    "manual_event": false,
                    "absolute": false,
                    "measure": []
                })
            }],
            inputs: vec![Data {
                name: "PH del agua".to_owned(),
                type_name: "Element".to_owned(),
                content: json!({
                    "element_name": "PH".to_owned(),
                    "element_type": "WaterFlow".to_owned(),
                    "category": "Water".to_owned(),
                    "unit": "pH".to_owned(),
                    "absolute": false,
                    "manual_event": false,
                    "measure": [],
                })
            }],
            other_outputs: vec![],
            scope: vec![Scopes::Temporal(25)],
        };

        let unit_process_2 = UnitProcess {
            name: "bottling of wine".to_owned(),
            main_outputs: vec![
                Data {
                    name: "Caudal volumetrico2 del aceite".to_owned(),
                    type_name: "Element".to_owned(),
                    content: json!({
                        "element_name": "Caudal volumetrico2".to_owned(),
                        "element_type": "WaterFlow2".to_owned(),
                        "category": "Caudal2".to_owned(),
                        "unit": "m^3/h2".to_owned(),
                        "measure": [],
                        "absolute": false,
                        "manual_event": false,
                    })
                }],
            inputs: vec![
                Data {
                    name: "PH2 del agua".to_owned(),
                    type_name: "Element".to_owned(),
                    content: json!({
                        "element_type": "WaterFlow2".to_owned(),
                        "category": "Water2".to_owned(),
                        "element_name": "PH2".to_owned(),
                        "unit": "pH2".to_owned(),
                        "measure": [],
                        "manual_event": false,
                        "absolute": false,
                    })
                }],
            other_outputs: vec![],
            scope: vec![Scopes::Temporal(33)],
        };

        let init_state = ProductionSystem {
            name: "wine process".to_owned(),
            version: 1,
            unit_process: vec![unit_process_1, unit_process_2],
            custom_types: types,
        };

        let context = sdk::Context {
            initial_state: init_state.clone(),
            event: Events::ChangeProductionSystem(ChangeProductionSystem::Modify {
                    delete: vec!["grape treading".to_owned(), "bottling of wine".to_owned()],
                    modification: vec![],
                    add: vec![],
                },
            ),
            is_owner: false,
        };

        let mut result = sdk::ContractResult::new(init_state);
        contract_logic(&context, &mut result);

        assert_eq!(result.final_state.version, 2);
        assert_eq!(result.final_state.unit_process.len(), 0);
        assert!(result.success);
    }

    #[test]
    fn test_change_operation_modify() {
        let mut custom_type_element = Fields(HashMap::new());
        custom_type_element.0.insert("element_type".to_owned(), DynamicType::String);
        custom_type_element.0.insert("category".to_owned(), DynamicType::String);
        custom_type_element.0.insert("element_name".to_owned(), DynamicType::String);
        custom_type_element.0.insert("unit".to_owned(), DynamicType::String);
        custom_type_element.0.insert("manual_event".to_owned(), DynamicType::bool);
        custom_type_element.0.insert("absolute".to_owned(), DynamicType::bool);
        custom_type_element.0.insert("measure".to_owned(), DynamicType::Vec(Box::new(DynamicType::Type("MeasureType".to_owned()))));
        let mut custom_type_scopes = Fields(HashMap::new());
        custom_type_scopes.0.insert("Temporal".to_owned(), DynamicType::u64);
        custom_type_scopes.0.insert("TimeZone".to_owned(), DynamicType::String);
        custom_type_scopes.0.insert("Tag".to_owned(), DynamicType::String);
        let mut custom_type_measure = Fields(HashMap::new());
        custom_type_measure.0.insert("value".to_owned(), DynamicType::f64);
        custom_type_measure.0.insert("scopes".to_owned(), DynamicType::Vec(Box::new(DynamicType::Enum(custom_type_scopes))));

        let mut types = HashMap::new();
        types.insert("Element".to_owned(), custom_type_element);
        types.insert("MeasureType".to_owned(), custom_type_measure);

        let unit_process_1 = UnitProcess {
            name: "grape treading".to_owned(),
            main_outputs: vec![Data {
                name: "Caudal volumetrico del aceite".to_owned(),
                type_name: "Element".to_owned(),
                content: json!({
                    "element_name": "Caudal volumetrico".to_owned(),
                    "element_type": "WaterFlow".to_owned(),
                    "category": "Caudal".to_owned(),
                    "unit": "m^3/h".to_owned(),
                    "manual_event": false,
                    "absolute": false,
                    "measure": []
                })
            }],
            inputs: vec![Data {
                name: "PH del agua".to_owned(),
                type_name: "Element".to_owned(),
                content: json!({
                    "element_name": "PH".to_owned(),
                    "element_type": "WaterFlow".to_owned(),
                    "category": "Water".to_owned(),
                    "unit": "pH".to_owned(),
                    "absolute": false,
                    "manual_event": false,
                    "measure": [],
                })
            }],
            other_outputs: vec![],
            scope: vec![Scopes::Temporal(25)],
        };

        let unit_process_2 = UnitProcess {
            name: "bottling of wine".to_owned(),
            main_outputs: vec![
                Data {
                    name: "Caudal volumetrico2 del aceite".to_owned(),
                    type_name: "Element".to_owned(),
                    content: json!({
                        "element_name": "Caudal volumetrico2".to_owned(),
                        "element_type": "WaterFlow2".to_owned(),
                        "category": "Caudal2".to_owned(),
                        "unit": "m^3/h2".to_owned(),
                        "measure": [],
                        "absolute": false,
                        "manual_event": false,
                    })
                }],
            inputs: vec![
                Data {
                    name: "PH2 del agua".to_owned(),
                    type_name: "Element".to_owned(),
                    content: json!({
                        "element_type": "WaterFlow2".to_owned(),
                        "category": "Water2".to_owned(),
                        "element_name": "PH2".to_owned(),
                        "unit": "pH2".to_owned(),
                        "measure": [],
                        "manual_event": false,
                        "absolute": false,
                    })
                }],
            other_outputs: vec![],
            scope: vec![Scopes::Temporal(33)],
        };

        let init_state = ProductionSystem {
            name: "wine process".to_owned(),
            version: 1,
            unit_process: vec![unit_process_1],
            custom_types: types,
        };

        let context = sdk::Context {
            initial_state: init_state.clone(),
            event: Events::ChangeProductionSystem(ChangeProductionSystem::Modify {
                    delete: vec![],
                    modification: vec![("grape treading".to_owned(), unit_process_2)],
                    add: vec![],
                },
            ),
            is_owner: false,
        };
        let mut result = sdk::ContractResult::new(init_state);
        contract_logic(&context, &mut result);

        assert!(result.error.is_empty());

        assert_eq!(result.final_state.unit_process[0].other_outputs.len(), 0);

        assert_eq!(result.final_state.unit_process[0].name, "bottling of wine");

        let element = result.final_state.unit_process[0].inputs[0].clone();
        assert_eq!(element.name, "PH2 del agua");
        assert_eq!(element.content, json!({
            "element_type": "WaterFlow2".to_owned(),
            "category": "Water2".to_owned(),
            "element_name": "PH2".to_owned(),
            "unit": "pH2".to_owned(),
            "measure": [],
            "manual_event": false,
            "absolute": false,
        }));

        let element = result.final_state.unit_process[0].main_outputs[0].clone();
        assert_eq!(element.name, "Caudal volumetrico2 del aceite");
        assert_eq!(element.content, json!({
            "element_name": "Caudal volumetrico2".to_owned(),
            "element_type": "WaterFlow2".to_owned(),
            "category": "Caudal2".to_owned(),
            "unit": "m^3/h2".to_owned(),
            "measure": [],
            "absolute": false,
            "manual_event": false,
        }));

        if let Scopes::Temporal(temporal) = result.final_state.unit_process[0].scope[0] {
            assert_eq!(temporal, 33);
        } else {
            panic!("Invalid Scope")
        }

        assert!(result.success);
    }

    #[test]
    fn test_change_register_data() {
        let mut custom_type_element = Fields(HashMap::new());
        custom_type_element.0.insert("element_type".to_owned(), DynamicType::String);
        custom_type_element.0.insert("category".to_owned(), DynamicType::String);
        custom_type_element.0.insert("element_name".to_owned(), DynamicType::String);
        custom_type_element.0.insert("unit".to_owned(), DynamicType::String);
        custom_type_element.0.insert("manual_event".to_owned(), DynamicType::bool);
        custom_type_element.0.insert("absolute".to_owned(), DynamicType::bool);
        custom_type_element.0.insert("measure".to_owned(), DynamicType::Vec(Box::new(DynamicType::Type("MeasureType".to_owned()))));
        let mut custom_type_scopes = Fields(HashMap::new());
        custom_type_scopes.0.insert("Temporal".to_owned(), DynamicType::u64);
        custom_type_scopes.0.insert("TimeZone".to_owned(), DynamicType::String);
        custom_type_scopes.0.insert("Tag".to_owned(), DynamicType::String);
        let mut custom_type_measure = Fields(HashMap::new());
        custom_type_measure.0.insert("value".to_owned(), DynamicType::f64);
        custom_type_measure.0.insert("scopes".to_owned(), DynamicType::Vec(Box::new(DynamicType::Enum(custom_type_scopes))));

        let mut types = HashMap::new();
        types.insert("Element".to_owned(), custom_type_element);
        types.insert("MeasureType".to_owned(), custom_type_measure);

        let unit_process_1 = UnitProcess {
            name: "grape treading".to_owned(),
            main_outputs: vec![Data {
                name: "Caudal volumetrico del aceite".to_owned(),
                type_name: "Element".to_owned(),
                content: json!({
                    "element_name": "Caudal volumetrico".to_owned(),
                    "element_type": "WaterFlow".to_owned(),
                    "category": "Caudal".to_owned(),
                    "unit": "m^3/h".to_owned(),
                    "manual_event": false,
                    "absolute": false,
                    "measure": []
                })
            }],
            inputs: vec![Data {
                name: "PH del agua".to_owned(),
                type_name: "Element".to_owned(),
                content: json!({
                    "element_name": "PH".to_owned(),
                    "element_type": "WaterFlow".to_owned(),
                    "category": "Water".to_owned(),
                    "unit": "pH".to_owned(),
                    "absolute": false,
                    "manual_event": false,
                    "measure": [],
                })
            }],
            other_outputs: vec![],
            scope: vec![Scopes::Temporal(25)],
        };

        let init_state = ProductionSystem {
            name: "wine process".to_owned(),
            version: 1,
            unit_process: vec![unit_process_1],
            custom_types: types,
        };

        let data_update = UnitData {
            scope: Some(vec![Scopes::Temporal(25)]),
            name: "grape treading".to_owned(),
            main_outputs: Some(vec![Data {
                name: "Caudal volumetrico del aceite".to_owned(),
                type_name: "Element".to_owned(),
                content: json!({
                    "element_name": "Caudal volumetrico".to_owned(),
                    "element_type": "WaterFlow".to_owned(),
                    "category": "Caudal".to_owned(),
                    "unit": "m^3/h".to_owned(),
                    "manual_event": false,
                    "absolute": false,
                    "measure": [{
                        "value": 0.5,
                        "scopes": [
                            {
                                "Temporal": 5555
                            },
                            {
                                "Tag": "IA"
                            },
                            {
                                "TimeZone": "UTC-0"
                            },
                        ]
                    }],
                })
            }]),
            inputs: Some(vec![Data {
                name: "PH del agua".to_owned(),
                type_name: "Element".to_owned(),
                content: json!({
                    "element_name": "PH".to_owned(),
                    "element_type": "WaterFlow".to_owned(),
                    "category": "Water".to_owned(),
                    "unit": "pH".to_owned(),
                    "absolute": false,
                    "manual_event": false,
                    "measure": [{
                        "value": 24.5,
                        "scopes": [
                            {
                                "Temporal": 515
                            },
                            {
                                "Tag": "Ledger"
                            },
                            {
                                "Tag": "IA"
                            },
                            {
                                "TimeZone": "UTC+1"
                            },
                        ]
                    }],
                })
            }]),
            other_outputs: None,
        };

        let context = sdk::Context {
            initial_state: init_state.clone(),
            event: Events::RegisterData(data_update),
            is_owner: false,
        };

        let mut result = sdk::ContractResult::new(init_state);
        contract_logic(&context, &mut result);

        assert_eq!(result.final_state.version, 1);
        assert_eq!(result.final_state.unit_process.len(), 1);
        assert_eq!(result.final_state.unit_process[0].name, "grape treading");
        assert_eq!(result.final_state.unit_process[0].other_outputs.len(), 0);

        let element = result.final_state.unit_process[0].inputs[0].clone();
        assert_eq!(element.name, "PH del agua");
        assert_eq!(element.content, json!({
            "element_name": "PH".to_owned(),
            "element_type": "WaterFlow".to_owned(),
            "category": "Water".to_owned(),
            "unit": "pH".to_owned(),
            "absolute": false,
            "manual_event": false,
            "measure": [{
                "value": 24.5,
                "scopes": [
                    {
                        "Temporal": 515
                    },
                    {
                        "Tag": "Ledger"
                    },
                    {
                        "Tag": "IA"
                    },
                    {
                        "TimeZone": "UTC+1"
                    },
                ]
            }],
        }));

        let element = result.final_state.unit_process[0].main_outputs[0].clone();
        assert_eq!(element.name, "Caudal volumetrico del aceite");
        assert_eq!(element.content, json!({
            "element_name": "Caudal volumetrico".to_owned(),
            "element_type": "WaterFlow".to_owned(),
            "category": "Caudal".to_owned(),
            "unit": "m^3/h".to_owned(),
            "manual_event": false,
            "absolute": false,
            "measure": [{
                "value": 0.5,
                "scopes": [
                    {
                        "Temporal": 5555
                    },
                    {
                        "Tag": "IA"
                    },
                    {
                        "TimeZone": "UTC-0"
                    },
                ]
            }],
        }));

        if let Scopes::Temporal(temporal) = result.final_state.unit_process[0].scope[0] {
            assert_eq!(temporal, 25);
        } else {
            panic!("Invalid Scope")
        }

        assert!(result.success);
    }
}
