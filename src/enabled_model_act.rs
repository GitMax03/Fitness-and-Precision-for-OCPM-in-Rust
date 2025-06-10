use std::collections::{HashMap, HashSet, VecDeque};
use counter::Counter;
use process_mining::ocel::ocel_struct::OCELEvent;
use crate::structs_for_ocpm::{Marking, OCPN};




//context to eventId: HasMap(context, eventIds of events with this context)
/*
pub fn get_enabled_model_activities(context: HashMap<String, HashMap<String, Counter<Vec<String>>>>,
                                    model: Model, bindings:  HashMap<(String,String), HashMap<String, Vec<String>>>, presets: HashMap<String, Vec<OCELEvent>>, context_to_eventId: HashMap<HashMap<String, HashMap<String, Counter<Vec<String>>>>, Vec<String>>) -> HashMap<String, Vec<String>> {


    /*
    TODO
    is it possible to group bindings into same sub-bindings => calc one sub-binding for all bindings with same sub-bindings?
    => better efficiency?
     */

    //iterate over each context
    for (context, event_ids) in context_to_eventId.iter() {

        //iterate over each event of this context
        for event_id in event_ids {




        }
    }

    None;
 }

 */




fn get_ebabled_model_activities_for_event(event_id: &String, preset: &Vec<OCELEvent>, bindings: &HashMap<(String,String), HashMap<String, Vec<String>>>, ocpn: &OCPN)  -> Vec<String> {
    //TODO: only use bindings with this event id??? => better efficency
    //TODO: preset must contain whole event?

    //get Binding sequence and used objects
    let (binding_sequence, used_objects) = get_binding_sequence_and_used_obj(preset, bindings);

    //queue of states: queue of <Place id, [object_ids]>
    let mut state_queue : VecDeque<Marking> = VecDeque::new();

    //create initial state and push it to the queue
    let initial_state = ocpn.initial_marking.clone().unwrap();
    state_queue.push_back(initial_state);
    
    while !state_queue.is_empty() {
        
        let current_state = state_queue.pop_front();
        
        if is_binding_fully_replayed(&binding_sequence.clone(), &current_state.clone().unwrap()){
            //TODO
        }
        
        //if next binding is enabled
        let next_binding = ocpn.find_first_enabled_binding(&current_state.clone().unwrap(), &binding_sequence);
        if next_binding.is_some() {
            //execute binding
            
        }
    }
    
    



    vec![]
}




//TODO: FALSE
//true if: all bindings are possible in the curretn state: i.e. all objects from all bindings are in the right places 
fn is_binding_fully_replayed(binding_sequence: &HashMap<String, Vec<Vec<String>>>, state: &Marking) -> bool {
    
    for (event_type, binding) in binding_sequence.iter() {
        //check if all objects of this binding are replayed int the state
        if let Some(objects) = state.get(event_type) {
            //check if all objects of the binding are in the state
            for object_ids in binding {
                if !object_ids.iter().all(|id| objects.contains(id)) {
                    //not all objects of this binding are in the state
                    return false; 
                }
            }
        } else {
            // no objects of this event type in the state => can't be fully replayed
            return false; 
        }
    }
    //all bindings are fully replayed in the state
    true
}


pub fn get_binding_sequence_and_used_obj(preset: &Vec<OCELEvent>, bindings: &HashMap<(String,String), HashMap<String, Vec<String>>>) -> (HashMap<String, Vec<Vec<String>>> ,HashSet<String>) {
    //not a ordered sequence


    //TODO: what are only visible transitions?
    //<event_type, [[object_ids (for first event with this event_type)], [object_ids (for second event with this event_type), ...]]>
    let mut binding_sequence: HashMap<String, Vec<Vec<String>>> = HashMap::new();
    //used objects
    let mut used_objects: HashSet<String> = HashSet::new();
    for event in preset {
        //push binding of event
        let binding = bindings.get(&(event.id.clone(), event.event_type.clone())).unwrap();
        binding_sequence.entry(event.event_type.clone())
            .or_insert_with(Vec::new)
            .push({
                let values: Vec<String> = binding.values().flatten().cloned().collect();
                used_objects.extend(values.clone());
                values
            }
            );
    }

    (binding_sequence, used_objects)
}

 