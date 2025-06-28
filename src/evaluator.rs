extern crate process_mining as pm;

use std::collections::{HashMap, HashSet};
use std::time::Instant;
use counter::Counter;


use crate::{enabled_log_act, NUMBER_OF_ITERATIONS};
use crate::enabled_model_act;
use crate::structs_for_ocpm::OCPN;
use crate::utils::evaluate_runtime;

//calc fitness and precision for given OCEL and OCPN
pub fn apply(ocel:pm::OCEL, ocpn: OCPN) -> (f64, f64) {
    let mut starting_times: Vec<Instant> = Vec::<Instant>::new();
    #[cfg(feature = "stats")]{
        println!("######## Evaluating with statistics ########");

        starting_times.push(Instant::now());
    }


    let events_ids: Vec<String> = ocel.events.iter().map(|event| event.id.clone()).collect();
    #[cfg(feature = "stats")]{
        let mut temp: Vec<String> = Vec::new();
    
        for i in 1..NUMBER_OF_ITERATIONS {
            temp = ocel.events.iter().map(|event| event.id.clone()).collect();
        }
        starting_times.push(Instant::now());
    }

    let (contexts, bindings) = enabled_log_act::get_contexts_and_bindings(&ocel);
    #[cfg(feature = "stats")]{
        for i in 1..NUMBER_OF_ITERATIONS {
            enabled_log_act::get_contexts_and_bindings(&ocel);
        }
        starting_times.push(Instant::now());
    }
    
    let presets = enabled_log_act::get_event_presets(&ocel); //TODO inefficient is calculated twice
    #[cfg(feature = "stats")]{
        for i in 1..NUMBER_OF_ITERATIONS {
            enabled_log_act::get_event_presets(&ocel);
        }
        starting_times.push(Instant::now());
    }
    
    
    let (enabled_log_activities, contexts_map) = enabled_log_act::get_enabled_log_activities(&ocel, &contexts);
    #[cfg(feature = "stats")]{
        for i in 1..NUMBER_OF_ITERATIONS {
            enabled_log_act::get_enabled_log_activities(&ocel, &contexts);
        }
        starting_times.push(Instant::now());
    }
    
    let enabled_model_activities = enabled_model_act::get_enabled_model_activities(&ocpn, &presets, &bindings, &contexts_map);
    #[cfg(feature = "stats")]{
        for i in 1..NUMBER_OF_ITERATIONS {
            enabled_model_act::get_enabled_model_activities(&ocpn, &presets, &bindings, &contexts_map);
        }
        starting_times.push(Instant::now());
    }

    
    
    
    let mut fitness: f64 = 0.0;
    let mut precision: f64 = 0.0;
    
    (fitness, precision) = calculate_fitness_and_precision(events_ids.clone(), &enabled_log_activities, &enabled_model_activities);
    #[cfg(feature = "stats")]{
        for i in 1..NUMBER_OF_ITERATIONS {
            calculate_fitness_and_precision(events_ids.clone(), &enabled_log_activities, &enabled_model_activities);
        }
        starting_times.push(Instant::now());
    }
    

    //evaluate runtime
    #[cfg(feature = "stats")]
    evaluate_runtime(&starting_times, &vec!["get event ids".to_string(), "get context and bindings".to_string(),
                                            "get presets".to_string(), "get enabled log activities".to_string(),
                                            "get enabled model activities".to_string(), "calculate fitness and precision".to_string()]);
    
    (fitness, precision)
}


fn calculate_fitness_and_precision(events_ids: Vec<String>, enabled_log_activities: &HashMap<String,HashSet<String>>, enabled_model_activities: &HashMap<String, HashSet<String>>) -> (f64,f64){
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
        //println!("Enabled log activities for event {}: {:?}", event_id, enabled_log_activities.get(event_id).unwrap());
        //print enabled model activities
        //println!("Enabled model activities for event {}: {:?}", event_id, enabled_model_activities.get(event_id).unwrap());

        fitness = fitness + numerator / fit_denominator;
        precision = precision + numerator / prec_denominator;
    }

    fitness = fitness / events_ids.len() as f64;
    precision = precision / (events_ids.len() - skiped_events ) as f64;
    
    (fitness, precision)
}



