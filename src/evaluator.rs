extern crate process_mining as pm;

use crate::enabled_log_act;
use crate::enabled_model_act;
use crate::structs_for_ocpm::OCPN;

//calc fitness and precision for given OCEL and OCPN
pub fn apply(ocel:pm::OCEL, ocpn: OCPN) -> (f64, f64){
    
    let events_ids:Vec<String> = ocel.events.iter().map(|event| event.id.clone()).collect();
    
    let (contexts, bindings) = enabled_log_act::get_contexts_and_bindings(&ocel);
    let presets = enabled_log_act::get_event_presets(&ocel); //TODO inefficient is calculated twice
    
    let (enabled_log_activities, contexts_map) = enabled_log_act::get_enabled_log_activities(&ocel, &contexts);
    let enabled_model_activities = enabled_model_act::get_enabled_model_activities(&ocpn, &presets, &bindings, &contexts_map);

    
    
    
    let mut fitness: f64 = 0.0;
    let mut precision: f64 = 0.0;
    
    let mut skiped_events = 0;
    
    
    
    for event_id in &events_ids {
        
        let numerator = enabled_log_activities.get(event_id).unwrap()
            .intersection(enabled_model_activities.get(event_id).unwrap()).count() as f64;
        
        let fit_denominator = enabled_log_activities.get(event_id).unwrap().len() as f64;
        let prec_denominator = enabled_model_activities.get(event_id).unwrap().len() as f64;
        
        if prec_denominator == 0.0 {
            skiped_events += 1;
        }

        //debug
        //print enabled log activities
        println!("Enabled log activities for event {}: {:?}", event_id, enabled_log_activities.get(event_id).unwrap());
        //print enabled model activities
        println!("Enabled model activities for event {}: {:?}", event_id, enabled_model_activities.get(event_id).unwrap());
        
        fitness = fitness + numerator / fit_denominator;
        precision = precision + numerator / prec_denominator;
    }
    
    fitness = fitness / events_ids.len() as f64;
    precision = precision / (events_ids.len() - skiped_events ) as f64;
    
    
    (fitness, precision)
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
