use crate::structs_for_ocpm::{hash_marking, Marking, OCPN};
use counter::Counter;
use process_mining::ocel::ocel_struct::OCELEvent;
use std::collections::{HashMap, HashSet, VecDeque};
use std::task::Context;
use process_mining::OCEL;
use rayon::prelude::*;
use crate::NUMBER_OF_ITERATIONS;

pub fn get_enabled_model_activities_(
    ocpn: &OCPN,
    bindings: &HashMap<String, Vec<(String, HashMap<String,HashSet<String>>)>>,
    contexts: &HashMap<String, HashMap<String, Counter<Vec<String>>>>,
    context_event_map: &HashMap<u64, HashSet<String>>, //hashed context, event_ids with this context
    all_object_types: &HashSet<String>,
    id_to_event: &HashMap<String, OCELEvent>,
) -> HashMap<String, HashSet<String>> { //event_id, enabled_model_activities
    

    let mut results: HashMap<String, HashSet<String>> = HashMap::new();

    //println!("Contexts map: {:?}", contexts_map);
    

    //iterate over each different context
    for (_, events) in context_event_map{
        #[cfg(feature = "progress-percentage")]{
            
            //print percentage every 5 iterations
            if results.len() % 5 == 0 {
                println!("Enabled Model act: {}%", (results.len() as f64 / id_to_event.len() as f64 * 100.0).round());
            }
        }
        
        
        //enabled model activities for this context
        //if: just for debug
        if false{
            let mut result: HashSet<String> = HashSet::new();
            for event_id in events{
                //calculate enabled model activities for this event
                //TODO use multithreading here!
                result.extend(get_enabled_model_activities_for_event(
                    event_id,
                    contexts.get(event_id).unwrap(),
                    bindings.get(event_id).unwrap(),
                    ocpn,
                    all_object_types,
                    id_to_event,
                ));
            }

            //TODO parallel insert and ArcHashSet
            //add result to results
            for event_id in events {
                results.insert(event_id.clone(), result.clone());
            }
            
        }else{
            //multithreaded
            let result: HashSet<String> = events
                .par_iter()
                .map(|event_id| {
                    get_enabled_model_activities_for_event(
                        event_id,
                        contexts.get(event_id).unwrap(),
                        bindings.get(event_id).unwrap(),
                        ocpn,
                        all_object_types,
                        id_to_event,
                    )
                })
                .reduce(
                    || HashSet::new(),
                    |mut res_set, res| {
                        res_set.extend(res);
                        res_set
                    },
                );
            //TODO parallel insert and ArcHashSet
            //add result to results
            for event_id in events {
                results.insert(event_id.clone(), result.clone());
            }
        }
          
        
    }
    
    
    results
}

pub fn get_enabled_model_activities_for_event(
    event_id: &String,
    context: &HashMap<String, Counter<Vec<String>>>, //context of event_id: object type: counter[activities]
    binding_: &Vec<(String, HashMap<String, HashSet<String>>)>, //map of binding sequences: activity, HashMap< object type: [objects]>
    ocpn: &OCPN,
    all_object_types: &HashSet<String>,
    id_to_event: &HashMap<String, OCELEvent>,
) -> HashSet<String> {

    let mut binding = binding_.clone(); //clone binding to avoid borrowing issues
    
    let mut needed_objects: HashMap<String,HashSet<String>> = HashMap::new(); //object_type: objects
    
    
    //get all associated objects
    for object_type in all_object_types {
        for (act, objs) in binding.iter() {
            for (ot, objects) in objs {
                if ot == object_type {
                    //add objects to needed_objects
                    needed_objects.entry(object_type.clone()).or_insert_with(HashSet::new).extend(objects.clone());
                }
            }
        }
    }
    //add objects from event 
    for relation in id_to_event.get(event_id).unwrap().relationships.iter() {

        let object_type = ocpn.object_to_type.get(&relation.object_id).unwrap();
        //add object to needed_objects
        needed_objects.entry(object_type.clone()).or_insert_with(HashSet::new).insert(relation.object_id.clone());
    }


    //add them to initial marking
    let mut initial_marking: Marking = Marking::new();
    //add initial marking: for each object type in context
    for object_type in context.keys(){
        let needed_objects_ = needed_objects.get(object_type).unwrap_or(&HashSet::new()).clone();

        //TODO additional objects?

        if needed_objects_.is_empty() { //if no objects of this type are needed, skip
            continue;
        }

        for (place_id, place) in ocpn.places.iter(){
            if place.object_type == *object_type && place.is_initial {
                //add objects to initial marking
                for object_id in needed_objects_.iter() {
                    initial_marking.entry(place_id.clone()).or_insert(HashSet::new()).insert(object_id.clone());
                }
                //TODO evt. add additional objects to initial marking
            }
        }
    }
    //--------------------------------------------
    //replay binding sequence:
    //queue of states: queue of <Place id, [object_ids]>
    let mut state_queue: VecDeque<Marking> = VecDeque::new();
    state_queue.push_back(initial_marking.clone()); //TODO: clone just for debug

    let mut result: HashSet<String> = HashSet::new(); //result: list of activities
    let mut seen_markings: HashMap<u64, usize> = HashMap::new(); //seen amrkings: <Hash of marking, len of binding> to avoid infinite loops
    
    while !state_queue.is_empty() {
        
        if state_queue.len() > 70_000 { println!("ABORT!!!"); break;}
        
        let current_state = state_queue.pop_front().unwrap();

        //if binding sequence is fully replayed in the current state: no bindinng is left in the sequence
        if binding.is_empty() {
            //add enabled transitions to result
            result.extend(ocpn.get_enabled_transitions_from_marking(&current_state)
                .into_iter()
                .map(|t_id| ocpn.transitions.get(&t_id).unwrap().activity.clone()));
        }
        //get next enabled binding
        if let Some(mut next_binding) = ocpn.remove_next_binding_(&mut binding, &current_state) {//if a binding is enabled, remove it
            // there is a binding enabled
            //execute next binding:
            let next_state = ocpn.execute_binding_(&mut next_binding, &current_state);
            //enqueue resulting state
            //only push if state new or binding len is different
            let hashed_next_state = hash_marking(&next_state);
            if let Some(len_of_seen_binding) = seen_markings.get(&hashed_next_state){
                if binding.len() == *len_of_seen_binding{ continue;}
            }
            seen_markings.insert(hashed_next_state.clone(), binding.len());
            state_queue.push_back(next_state);
            
        }else{ //there is no enabled binding
            //check for any enabled silent transitions
            let enabled_silent_bindings = ocpn.get_enabled_silent_transitions_bindings_(&current_state);
            //let enabled_silent_bindings = ocpn.get_enabled_silent_transition_bindings(&current_state);

            //execute all enabled silent transitions
            for mut enabled_binding in enabled_silent_bindings {
                //execute binding
                let state = ocpn.execute_binding_(&mut enabled_binding, &current_state); //TODO check func

                //enqueue resulting state
                //only push if state new or binding len is different
                let hashed_next_state = hash_marking(&state);
                if let Some(len_of_seen_binding) = seen_markings.get(&hashed_next_state){
                    if binding.len() == *len_of_seen_binding{ continue;}
                }
                seen_markings.insert(hashed_next_state.clone(), binding.len());
                state_queue.push_back(state);
            }
        }
    }
    /*
    if result.is_empty(){
        println!("No enabled transitions found");
    }
    
     */
    //print!(".");
    result
}

