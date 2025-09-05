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
            return Err(format!("A Cycle is detected in {} type", type_name));
        }
    }

    Ok(())
}

fn add_types(state: &mut ProductionSystem, types: Vec<(String, Fields)>) -> Result<(), String> {
    if types.is_empty() {
        return Err("Types can not be a empty Vec".to_owned());
    }

    let mut temporal_types: HashMap<String, Fields> = state.custom_types.clone();

    for (name, fields) in types.clone() {
        temporal_types.insert(name, fields);
    }

    let mut cycle_types: HashMap<String, Vec<String>> = HashMap::new();

    for (name, fields) in temporal_types.clone() {
        if name.is_empty() {
            return Err("Type name can not be empty".to_owned());
        }
        match name.as_str() {
            "String" | "bool" | "i64" | "f64" | "u64" | "Dummy" | "Option" | "Enum" | "Type"
            | "Vec" => {
                return Err(format!("The type name is reserved, name: {}", name));
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
        return Err("Unit process can not be a empty Vec".to_owned());
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
        return Err("Units procces has not uniques names".to_owned());
    }

    Ok(())
}

fn add_new_propierties(
    state: &mut ProductionSystem,
    propierties: Vec<Propierties>,
) -> Result<(), String> {
    if propierties.is_empty() {
        return Err("Porduction system add propierties can not be a empty Vec".to_owned());
    }
    for pro in propierties.clone() {
        pro.check_data(&state.custom_types)?;
        state.propierties.push(pro);
    }

    let propierties_names = state
        .propierties
        .iter()
        .map(|x| x.name.clone())
        .collect::<Vec<String>>();
    let hash_propierties_names: HashSet<String> =
        HashSet::from_iter(propierties_names.iter().cloned());

    if hash_propierties_names.len() != propierties_names.len() {
        return Err("Units procces has not uniques names".to_owned());
    }

    Ok(())
}

fn check_data(
    type_name: &str,
    content: Value,
    custom_types: &HashMap<String, Fields>,
) -> Result<(), String> {
    if type_name.is_empty() {
        return Err("Element type name can not be empty".to_owned());
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
            _ => Err("Element type name can not be empty".to_owned()),
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
        return Err("Data name is not the same".to_owned());
    }

    if local_type_name != type_name {
        return Err("Data type is not the same".to_owned());
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
            _ => return Err("Element type name can not be empty".to_owned()),
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
    pub propierties: Vec<Propierties>,
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
                    return Err("Basic type can not be Dummy, Option or Type".to_owned());
                }

                dynamic_type.check_data(custom_types.clone(), &mut internal_types)?;
            }
            Fields::Object(hash_map) => {
                if hash_map.is_empty() {
                    return Err("Fields can not be empty".to_owned());
                }

                for (field, c_type) in hash_map.iter() {
                    if let DynamicType::Dummy = c_type {
                        return Err("Object type can not be Dummy".to_owned());
                    }
                    if field.is_empty() {
                        return Err("Field can not be empty".to_owned());
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
                    return Err("Data can not convert in Object".to_owned());
                };

                let (len, options) = count_options(hash_map);
                if data_object.len() < len - options || data_object.len() > len {
                    return Err("Data fields and type fields is not the same".to_owned());
                }

                for (custom_type_name, custom_type_type) in hash_map.clone() {
                    if let Some(field_type) = data_object.remove(&custom_type_name) {
                        custom_type_type.deserialize(field_type, custom_types)?;
                    } else if !custom_type_type.is_option() {
                        return Err(format!(
                            "A field in type do not exist in data: {}",
                            custom_type_name
                        ));
                    };
                }

                if !data_object.is_empty() {
                    return Err("Data has more fields than type".to_owned());
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
                    vec_type.deserialize(val, custom_types)?;
                }
            }
            DynamicType::Enum(enum_type) => {
                if let Some(obj_dynamic) = value.as_object().cloned() {
                    if obj_dynamic.len() != 1 {
                        return Err(
                            "Can not deserialize, in Enum Object must have one field".to_owned()
                        );
                    }

                    for (value_name, value_val) in obj_dynamic {
                        let Some(type_dyn) = enum_type.get(&value_name) else {
                            return Err("Can not deserialize, Value does not match enum".to_owned());
                        };

                        type_dyn.deserialize(value_val, custom_types)?;
                    }
                } else if let Some(obj_dynamic) = value.as_str() {
                    if let Some(DynamicType::Dummy) = enum_type.get(obj_dynamic) {
                        // Ok
                    } else {
                        return Err("Can not deserialize, Value does not match enum".to_owned());
                    }
                } else {
                    return Err("Can not deserialize Value as Enum".to_owned());
                };
            }
            DynamicType::Type(c_type) => {
                let Some(obj_type) = custom_types.get(c_type) else {
                    return Err(format!(
                        "Can not deserialize, Custom type {} does not exist",
                        c_type
                    ));
                };

                match obj_type {
                    Fields::Basic(type_dyn) => {
                        type_dyn.deserialize(value, custom_types)?;
                    }
                    Fields::Object(hash_map) => {
                        let Some(mut obj_dynamic) = value.as_object().cloned() else {
                            return Err("Can not deserialize Value as Object".to_owned());
                        };

                        let (len, options) = count_options(hash_map);
                        if obj_dynamic.len() < len - options || obj_dynamic.len() > len {
                            return Err(
                                "Can not deserialize, Value has diferents fields than Object"
                                    .to_owned(),
                            );
                        }

                        for (type_field, type_dyn) in hash_map.clone() {
                            if let Some(value) = obj_dynamic.remove(&type_field) {
                                type_dyn.deserialize(value, custom_types)?;
                            } else if !type_dyn.is_option() {
                                return Err(format!(
                                    "Can not deserialize, Value has not {} field",
                                    type_field
                                ));
                            };
                        }

                        if !obj_dynamic.is_empty() {
                            return Err(
                                "Can not deserialize, Value has more fields than Type".to_owned()
                            );
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
                return Err("Invalid Dummy type".to_owned());
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
                    return Err("Vec or Option type can not be Dummy or Option".to_owned());
                }
                c_type.check_data(custom_types, internal_types)?;
            }
            DynamicType::Enum(enum_type) => {
                for (type_field, type_dyn) in enum_type.clone() {
                    if type_field.is_empty() {
                        return Err("Field can not be empty".to_owned());
                    }

                    if let DynamicType::Option(..) = type_dyn {
                        return Err("In Enum a field can not be option".to_owned());
                    }

                    type_dyn.check_data(custom_types.clone(), internal_types)?;
                }
            }
            DynamicType::Type(c_type) => {
                if c_type.is_empty() {
                    return Err("Custom type con not be empty".to_string());
                }

                if !custom_types.contains_key(c_type) {
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
    pub outputs: Option<Vec<RegisterData>>,
    pub inputs: Option<Vec<RegisterData>>,
    pub propierties: Option<Vec<Propierties>>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct UnitProcess {
    pub name: String,
    pub inputs: Vec<Data>,
    pub outputs: Vec<Data>,
    pub propierties: Vec<Propierties>,
}

impl UnitProcess {
    pub fn check_data(&self, custom_types: &HashMap<String, Fields>) -> Result<(), String> {
        if self.name.is_empty() {
            return Err("Unit process name can not be empty".to_owned());
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
            return Err("Inputs and outputs has not unique names".to_owned());
        }

        let mut propierties_name = vec![];
        for p in self.propierties.iter() {
            p.check_data(custom_types)?;
            propierties_name.push(p.name.clone());
        }

        let hash_pro_name: HashSet<String> = HashSet::from_iter(propierties_name.iter().cloned());
        if hash_name.len() != self.outputs.len() + self.inputs.len() {
            return Err("Inputs and outputs has not unique names".to_owned());
        }

        if hash_pro_name.len() != self.propierties.len() {
            return Err("Propierties has not unique names".to_owned());
        }

        Ok(())
    }

    pub fn register_data(
        &mut self,
        unit: UnitData,
        custom_types: &HashMap<String, Fields>,
    ) -> Result<(), String> {
        if unit.inputs.is_none() && unit.outputs.is_none() {
            return Err("Inputs and outputs are empty".to_owned());
        }

        if let Some(inputs) = unit.inputs {
            if inputs.is_empty() {
                return Err("Inputs can not be Some and be empty".to_owned());
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
                return Err(
                    "An attempt was made to update inputs that do not exist in the unit process"
                        .to_owned(),
                );
            }
        }

        if let Some(outputs) = unit.outputs {
            if outputs.is_empty() {
                return Err("outputs can not be Some and be empty".to_owned());
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
                return Err(
                    "An attempt was made to update Outputs that do not exist in the unit process"
                        .to_owned(),
                );
            }
        }

        if let Some(propierties) = unit.propierties {
            if propierties.is_empty() {
                return Err("propierties can not be Some and be empty".to_owned());
            }

            let mut updates: usize = 0;

            for data in self.propierties.iter_mut() {
                for unit_data in propierties.clone() {
                    if data.name == unit_data.name {
                        data.register_data(unit_data, custom_types)?;
                        updates += 1;
                    }
                }
            }

            if updates != propierties.len() {
                return Err(
                    "An attempt was made to update propierties that do not exist in the unit process"
                        .to_owned(),
                );
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
struct Propierties {
    pub name: String,
    pub type_name: String,
    pub content: Value,
}

impl Propierties {
    fn check_data(&self, custom_types: &HashMap<String, Fields>) -> Result<(), String> {
        if self.name.is_empty() {
            return Err("Element name can not be empty".to_owned());
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
            return Err("In unit process definition targets must be None".to_owned());
        }

        if self.name.is_empty() {
            return Err("Element name can not be empty".to_owned());
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
                    return Err("In Data Target governance_id, subject_id and unit_procees can not be empty".to_owned());
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
        propierties: Option<Vec<Propierties>>,
    },
    // TODO REHACER
    ModifyProductionSystem {
        name: Option<String>,
        delete_propierties: Option<Vec<String>>,
        modify_propierties: Option<Vec<(String, Propierties)>>,
        add_propierties: Option<Vec<Propierties>>,
    },
    // TODO HACER UNO DONDE SE BORRE
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
            contract_result.error = "Can not emit init event if version is != 0".to_owned();
            return;
        }
    } else if state.version == 0 {
        contract_result.error = "The first event must be Init event".to_owned();
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
                propierties,
                types,
            } => {
                if name.is_empty() {
                    contract_result.error = "System name can not be empty".to_owned();
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

                if let Some(propierties) = propierties {
                    if let Err(e) = add_new_propierties(state, propierties) {
                        contract_result.error = e;
                        return;
                    }
                }
            }
            ChangeProductionSystem::ModifyProductionSystem {
                name,
                delete_propierties,
                add_propierties,
                modify_propierties,
            } => {
                if name.is_none()
                    && delete_propierties.is_none()
                    && add_propierties.is_none()
                    && modify_propierties.is_none()
                {
                    contract_result.error = "In ModifyProductionSystem name, delete_propierties, add_propierties and modify_propierties can not be None".to_owned();
                    return;
                }

                if let Some(name) = name {
                    if name.is_empty() {
                        contract_result.error = "New name can not be empty".to_owned();
                        return;
                    }

                    state.name = name;
                }

                if let Some(delete_propierties) = delete_propierties {
                    if delete_propierties.is_empty() {
                        contract_result.error =
                            "Porduction system delete propierties can not be a empty Vec"
                                .to_owned();
                        return;
                    }

                    for name in delete_propierties.iter() {
                        if let Some(pos) = state.propierties.iter().position(|x| x.name == *name) {
                            state.propierties.remove(pos);
                        } else {
                            contract_result.error = format!(
                                "The Production System propiertie to be eliminated {} does not exist in the production system",
                                name
                            );
                            return;
                        }
                    }
                }

                if let Some(modify_propierties) = modify_propierties {
                    if modify_propierties.is_empty() {
                        contract_result.error =
                            "Porduction system modify propierties can not be a empty Vec"
                                .to_owned();
                        return;
                    }

                    for (name, propiertie) in modify_propierties.clone() {
                        if let Err(e) = propiertie.check_data(&state.custom_types) {
                            contract_result.error = e;
                            return;
                        };

                        if let Some(existing) =
                            state.propierties.iter_mut().find(|x| x.name == name)
                        {
                            *existing = propiertie;
                        } else {
                            contract_result.error = format!(
                                "The propiertie to be modificated {} does not exist in the production system",
                                name
                            );
                            return;
                        }
                    }
                }

                if let Some(add_propierties) = add_propierties {
                    if let Err(e) = add_new_propierties(state, add_propierties) {
                        contract_result.error = e;
                        return;
                    }
                }
            }
            ChangeProductionSystem::ModifyTypes { delete, add } => {
                if delete.is_none() && add.is_none() {
                    contract_result.error =
                        "In ModifyTypes add and delete can not be None".to_owned();
                    return;
                }

                if let Some(delete) = delete {
                    for name in delete {
                        if state.custom_types.remove(&name).is_none() {
                            contract_result.error =
                                format!("The type to be deleted does not exist, type: {}", name);
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
                    contract_result.error =
                        "In ModifyUnitProcess add, modify and delete can not be None".to_owned();
                    return;
                }

                if let Some(delete) = delete {
                    if delete.is_empty() {
                        contract_result.error =
                            "Delete unit process can not be a empty Vec".to_owned();
                        return;
                    }

                    for name in delete.clone() {
                        if let Some(pos) = state.unit_process.iter().position(|x| x.name == name) {
                            state.unit_process.remove(pos);
                        } else {
                            contract_result.error = format!(
                                "The process to be eliminated {} does not exist in the production system",
                                name
                            );
                            return;
                        }
                    }
                }

                if let Some(modify) = modify {
                    if modify.is_empty() {
                        contract_result.error =
                            "Modify unit process can not be a empty Vec".to_owned();
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
                                "The process to be modificated {} does not exist in the production system",
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
                contract_result.error = "The first event must be Init event".to_owned();
                return;
            }

            if data.is_empty() {
                contract_result.error = "Register data can not be empty".to_owned();
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
                    contract_result.error = format!("No processing unit found matching {}", d.name);
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
        contract_logic, ChangeProductionSystem, Data, DynamicType, Events, Fields, Metadata, ProductionSystem, Propierties, RegisterData, Target, UnitData, UnitProcess
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

    

    impl PartialEq for Propierties {
        fn eq(&self, other: &Self) -> bool {
            self.name == other.name
                && self.type_name == other.type_name
                && self.content == other.content
        }
    }
    
    impl Eq for Propierties {}

    impl PartialEq for UnitProcess {
        fn eq(&self, other: &Self) -> bool {
            self.name == other.name
                && self.inputs == other.inputs
                && self.outputs == other.outputs
                && self.propierties == other.propierties
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
            propierties: vec![],
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
            propierties: vec![],
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
            propierties: vec![],
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
            propierties: vec![],
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
            propierties: vec![],
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
            propierties: vec![],
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
            propierties: vec![],
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
            propierties: vec![],
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
            propierties: vec![],
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
            propierties: vec![],
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
            propierties: vec![],
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
            propierties: vec![],
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
            propierties: vec![],
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
            propierties: vec![],
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
            propierties: vec![],
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
            propierties: vec![],
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
            propierties: vec![],
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
            propierties: vec![],
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
            propierties: vec![],
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
                propierties: None,
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
            propierties: vec![],
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
            propierties: vec![],
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
            propierties: vec![],
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
            propierties: vec![],
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
            propierties: vec![],
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
            propierties: vec![],
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
            propierties: vec![],
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
            propierties: vec![],
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
            propierties: vec![],
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
            propierties: vec![],
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
            propierties: vec![],
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
            propierties: vec![],
        };

        let context = sdk::Context {
            event: Events::ChangeProductionSystem(ChangeProductionSystem::ModifyProductionSystem {
                name: Some("wine process 2".to_owned()),
                delete_propierties: None,
                modify_propierties: None,
                add_propierties: None,
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
            propierties: vec![],
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
                    propierties: vec![Propierties {
                        name: "Example String".to_owned(),
                        type_name: "String".to_owned(),
                        content: json!("ExampleString"),
                    }],
                }]),
                propierties: Some(vec![Propierties {
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
        assert_eq!(result.state.propierties.len(), 1);
        assert_eq!(
            result.state.propierties[0],
            Propierties {
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
                propierties: vec![Propierties {
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
            propierties: vec![],
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
            propierties: vec![],
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
            propierties: vec![],
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
