extern crate process_mining as pm;

use std::collections::{HashMap, HashSet};
use std::time::Instant;
use counter::Counter;
use process_mining::ocel::ocel_struct::OCELEvent;
use crate::{enabled_log_act, LOG_MODEL, NUMBER_OF_ITERATIONS, VERSION_NAME};
use crate::enabled_model_act;
use crate::structs_for_ocpm::OCPN;
use crate::utils::{evaluate_runtime, save_analytics};

//calc fitness and precision for given OCEL and OCPN
pub fn apply(ocel:pm::OCEL, ocpn: OCPN) -> (f64, f64) {
    let mut starting_times: Vec<Instant> = Vec::<Instant>::new();
    #[cfg (feature = "stats")]{
        println!("######## Evaluating with statistics ########");

        starting_times.push(Instant::now());
    }

    println!("getting event ids, object types and id to event map ...");
    let events_ids: Vec<String> = ocel.events.iter().map(|event| event.id.clone()).collect();
    let all_object_types: HashSet<String> = ocel.object_types.iter().map(|ot| ot.name.clone()).collect();
    let id_to_event: HashMap<String, OCELEvent> = ocel.events.iter().map(|e| (e.id.clone(), e.clone())).collect();
    #[cfg(feature = "stats")]{
        let mut temp: Vec<String> = Vec::new();
        let mut temp2: HashSet<String> = HashSet::new();
        let mut temp3: HashMap<String, OCELEvent> = HashMap::new();
    
        for i in 1..NUMBER_OF_ITERATIONS {
            temp = ocel.events.iter().map(|event| event.id.clone()).collect();
            temp2 = ocel.object_types.iter().map(|ot| ot.name.clone()).collect();
            temp3 = ocel.events.iter().map(|e| (e.id.clone(), e.clone())).collect();
        }
        starting_times.push(Instant::now());
    }
    println!("calculating presets ...");
    let presets = enabled_log_act::get_event_presets(&ocel, &id_to_event); //TODO inefficient is calculated twice
    #[cfg(feature = "stats")]{
        for i in 1..NUMBER_OF_ITERATIONS {
            enabled_log_act::get_event_presets(&ocel, &id_to_event);
        }
        starting_times.push(Instant::now());
    }
    println!("calculating contexts and bindings ...");
    let (contexts, bindings) = enabled_log_act::_get_contexts_and_bindings(&ocel, &id_to_event, &Some(presets.clone()), &all_object_types, &ocpn.object_to_type);
//TODO TODO context empty????????????? HILFE
    #[cfg(feature = "stats")]{
        for i in 1..NUMBER_OF_ITERATIONS {
            enabled_log_act::_get_contexts_and_bindings(&ocel, &id_to_event, &Some(presets.clone()), &all_object_types, &ocpn.object_to_type);
        }
        starting_times.push(Instant::now());
    }
    //debug
    //let mut contexts_: Vec<_> = contexts.iter().collect();
    //contexts_.sort_by_key(|(k, _)| k.parse::<u32>().unwrap_or(0));
    
    println!("calculating enabled log activities ...");
    let (enabled_log_activities, context_map) = enabled_log_act::get_enabled_log_activities(&contexts, &id_to_event);
    #[cfg(feature = "stats")]{
        for i in 1..NUMBER_OF_ITERATIONS {
            enabled_log_act::get_enabled_log_activities(&contexts, &id_to_event);
        }
        starting_times.push(Instant::now());
    }
    println!("calculating enabled model activities ...");
    let enabled_model_activities = enabled_model_act::get_enabled_model_activities_(&ocpn, &bindings, &contexts, &context_map,  &all_object_types, &id_to_event);
    //let enabled_model_activities = enabled_model_act::get_enabled_model_activities(&ocpn, &ocel, &presets, &bindings, &contexts_map);
    #[cfg(feature = "stats")]{
        for i in 1..NUMBER_OF_ITERATIONS {
            enabled_model_act::get_enabled_model_activities_(&ocpn, &bindings, &contexts, &context_map,  &all_object_types, &id_to_event);
        }
        starting_times.push(Instant::now());
    }

    
    println!("calculating fitness and precision ...");
    
    let (fitness, precision) = calculate_fitness_and_precision(events_ids.clone(), &enabled_log_activities, &enabled_model_activities);
    #[cfg(feature = "stats")]{
        for i in 1..NUMBER_OF_ITERATIONS {
            calculate_fitness_and_precision(events_ids.clone(), &enabled_log_activities, &enabled_model_activities);
        }
        starting_times.push(Instant::now());
    }
    

    //evaluate runtime
    #[cfg(feature = "stats")]
    evaluate_runtime( &starting_times, &vec!["get event ids".to_string(), "get presets".to_string(), "get context and bindings".to_string(),
                                             "get enabled log activities".to_string(),
                                            "get enabled model activities".to_string(), "calculate fitness and precision".to_string()]);
    #[cfg(feature = "save_stats")]
    save_analytics(VERSION_NAME.to_string(), LOG_MODEL.to_string(), &starting_times, &vec!["get event ids".to_string(), "get presets".to_string(), "get context and bindings".to_string(),
                                                             "get enabled log activities".to_string(),
                                                            "get enabled model activities".to_string(), "calculate fitness and precision".to_string()]);
    
    
    (fitness, precision)
}


fn calculate_fitness_and_precision(events_ids: Vec<String>, enabled_log_activities: &HashMap<String,HashSet<String>>, enabled_model_activities: &HashMap<String, HashSet<String>>) -> (f64,f64){
    let mut fitness: f64 = 0.0;
    let mut precision: f64 = 0.0;

    let mut skiped_events = 0;
    
    /*
    for event_id in &events_ids {

        let numerator = enabled_log_activities.get(event_id).unwrap()
            .intersection(enabled_model_activities.get(event_id).unwrap()).count() as f64;

        let fit_denominator = enabled_log_activities.get(event_id).unwrap().len() as f64;
        let prec_denominator = enabled_model_activities.get(event_id).unwrap().len() as f64;
        
        if fit_denominator == 0.0 || prec_denominator == 0.0{
            skiped_events += 0;
        }

        if prec_denominator == 0.0 {
            skiped_events += 1;
        }

        //debug
        //print enabled log activities
        //println!("Enabled log activities for event {}: {:?}", event_id, enabled_log_activities.get(event_id).unwrap());
        //print enabled model activities
        //println!("Enabled model activities for event {}: {:?}", event_id, enabled_model_activities.get(event_id).unwrap());

        /*
        println!("Event id: {}", event_id);
        println!("Numerator: {}", numerator);
        println!("Fit denominator: {}", fit_denominator);
        println!("Prec denominator: {}", prec_denominator);
        println!("-----------------------------------------");
        
         */

        fitness = fitness + numerator / fit_denominator;
        precision = precision + numerator / prec_denominator;
    }

    fitness = fitness / events_ids.len() as f64;
    precision = precision / (events_ids.len() - skiped_events ) as f64;
    
    (fitness, precision)
    
     */
    
    
    
    //-----
    let mut precisions: Vec<f64> = Vec::new();
    let mut fitnesses: Vec<f64> = Vec::new();
    let mut skipped_events: usize = 0;

    for event_id in &events_ids {
        let en_log_act = enabled_log_activities.get(event_id).unwrap();
        let en_model_act = enabled_model_activities.get(event_id).unwrap();
        
        if en_model_act.is_empty() || en_model_act.intersection(en_log_act).count() == 0 {
            skipped_events += 1;
            fitnesses.push(0.0);
            continue; // Skip this event if either set is empty
        }
        
        precisions.push(
            en_log_act.intersection(en_model_act).count() as f64 / en_model_act.len() as f64
        );
        fitnesses.push(
            en_log_act.intersection(en_model_act).count() as f64 / en_log_act.len() as f64
        );
    }
    
    if fitnesses.len() == 0{
        println!("fitnesses is empty, returning (0.0, 0.0)");
        return (0.0,0.0);
    }
    if precisions.len() == 0{
        println!("precisions is empty");
        return (fitnesses.iter().sum::<f64>() / fitnesses.len() as f64, 0.0);
    }
    (fitnesses.iter().sum::<f64>() / fitnesses.len() as f64, precisions.iter().sum::<f64>() / precisions.len() as f64)
    
}



