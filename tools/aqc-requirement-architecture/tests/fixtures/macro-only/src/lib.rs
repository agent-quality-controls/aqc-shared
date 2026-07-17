use external_membership::EngineRequirement;

macro_rules! parameterized_requirement {
    ($contract:path, $name:ident) => {
        pub struct $name;
        impl $contract for $name {}
    };
}

parameterized_requirement!(EngineRequirement, ParameterizedRequirement);
