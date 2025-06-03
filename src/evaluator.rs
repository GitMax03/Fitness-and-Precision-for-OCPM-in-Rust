extern crate process_mining as pm;

//calc fitness and precision for given OCEL and OCPN
fn apply(ocel:pm::OCEL) -> (f32, f32){

    
    //TODO: Context and Bindings
    (1.0, 1.0)
}

/*
Python Code to translate:

    object_types = ocel.object_types
    if contexts == None or bindings == None:
        contexts, bindings = utils.calculate_contexts_and_bindings(ocel)
    en_l =  replay_context.enabled_log_activities(ocel.log, contexts)
    en_m =  replay_context.enabled_model_activities_multiprocessing(contexts, bindings, ocpn, object_types)
    precision, skipped_events, fitness =  replay_context.calculate_precision_and_fitness(ocel.log, contexts, en_l, en_m)
    return precision, fitness


 */
