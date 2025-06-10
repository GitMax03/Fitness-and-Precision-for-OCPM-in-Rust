mod enabled_log_act;
mod evaluator;
mod structs_for_ocpm;
mod test;
mod utils;
mod enabled_model_act;

fn main() {
    
    test::test_enabled_log_activities();
    test::test_context_and_bindings();
    test::test_event_presets();
    test::test_binding_sequence();
    //test::test_eog();
}
