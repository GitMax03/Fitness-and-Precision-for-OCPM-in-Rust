use crate::structs_for_ocpm::{Marking, OCPN};
use counter::Counter;
use process_mining::ocel::ocel_struct::OCELEvent;
use std::collections::{HashMap, HashSet, VecDeque};
use std::task::Context;
use process_mining::OCEL;
use rayon::prelude::*;
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

pub fn get_enabled_model_activities(
    ocpn: &OCPN,
    ocel: &OCEL,
    presets: &HashMap<String, Vec<OCELEvent>>,
    bindings: &HashMap<(String, String), HashMap<String, Vec<String>>>,
    contexts_map: &HashMap<u64, (HashSet<String>, HashSet<String>)>
) -> HashMap<String, HashSet<String>> {

    //TODO: efficency could probably be better

    let mut result: HashMap<String, HashSet<String>> = HashMap::new();
    
    //println!("Contexts map: {:?}", contexts_map);

    //iterate over each different context
    for (events,_) in contexts_map.values() {
        let mut inter_result: HashSet<String> = HashSet::new();
        
        //iterate over each event in the context
        for event_id in events {
            
            //get preset for this event
            let enabled_model_activities_for_event = get_ebabled_model_activities_for_event(
                event_id,
                ocel,
                presets.get(event_id).unwrap(),
                bindings,
                ocpn,
            );
            
            //add enabled model activities for this event to the inter_result
            inter_result.extend(enabled_model_activities_for_event);
        }
        
        //add inter_result to result
        for event_id in events {
            result.insert(event_id.to_string(), inter_result.clone());
        }
        
        inter_result.clear();
    }
    result
}

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
        //enabled model activities for this context
        
        /*
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
         */
        
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
            
         
            
         
        
        

        //add result to results
        for event_id in events {
            results.insert(event_id.to_string(), result.clone());
        }
    }
    
    
    results
}

fn get_initial_marking(
    ocpn: &OCPN,
    event_id: &String,
    binding_sequence: &HashMap<String, Vec<Vec<String>>>,
    ocel: &OCEL,
) -> Marking {

    let mut initial_state: Marking = Marking::new();
    let event = ocel.events.iter().filter(|e| e.id == *event_id).next().unwrap();
    println!("should not be used!!!");
    for binding in binding_sequence.clone() {
        //TODO what if there are multiple transitions with same activity and input places?
        let temp_transition_id = ocpn.activity_to_transitions.get(&binding.0).unwrap().first().clone().unwrap();
        let possible_input_places = ocpn.transitions.get(temp_transition_id).unwrap().input_places.clone();
        for (input_place, object_type) in possible_input_places {
            let place = ocpn.places.get(&input_place).unwrap();
            if place.is_initial{
                for objects in binding.1.clone() {
                    for object_id in objects{
                        //add object to initial state
                        if place.object_type == *ocpn.object_to_type.get(&object_id.clone()).unwrap() {
                            initial_state.entry(input_place.clone()).or_insert(HashSet::new()).insert(object_id.clone());
                        }
                    }

                }
            }
        }
    }
    if binding_sequence.is_empty() {
        let input_places = ocpn.transitions.get(ocpn.activity_to_transitions.get(&event.event_type.clone()).unwrap().first().unwrap()).unwrap().input_places.clone();
        for (place_id, object_type) in input_places {
            for relation in event.relationships.iter() {
                if ocpn.object_to_type.get(&relation.object_id).unwrap() == &object_type {
                    //add object to initial state
                    initial_state.entry(place_id.clone()).or_insert(HashSet::new()).insert(relation.object_id.clone());
                }
            }
        }
    }
    initial_state
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
    state_queue.push_back(initial_marking);

    let mut result: HashSet<String> = HashSet::new(); //result: list of activities

    while !state_queue.is_empty() {
        let current_state = state_queue.pop_front().unwrap();

        //if binding sequence is fully replayed in the current state: no bindinng is left in the sequence
        if binding.is_empty() {
            //add enabled transitions to result
            result.extend(ocpn.get_enabled_transitions_from_marking(&current_state));
        }
        //get next enabled binding
        if let Some(mut next_binding) = ocpn.remove_next_binding_(&mut binding, &current_state) {//if a binding is enabled, remove it
            // there is a binding enabled
            //execute next binding:
            let next_state = ocpn.execute_binding_(&mut next_binding, current_state);
            //enqueue resulting state
            state_queue.push_back(next_state);
        }else{ //there is no enabled binding
            //check for any enabled silent transitions
            let enabled_silent_bindings = ocpn.get_enabled_silent_transitions_bindings_(&current_state);
            //let enabled_silent_bindings = ocpn.get_enabled_silent_transition_bindings(&current_state);

            //execute all enabled silent transitions
            for enabled_binding in enabled_silent_bindings {
                //execute binding
                let state = ocpn.execute_binding_(&mut enabled_binding.clone(), current_state.clone()); //TODO check func

                //enqueue resulting state
                state_queue.push_back(state);
            }
        }
    }

    
        
    
    
    result
}


///ocpn must have initial marking set!!!
pub fn get_ebabled_model_activities_for_event(
    event_id: &String,
    ocel: &OCEL,
    preset: &Vec<OCELEvent>,
    bindings: &HashMap<(String, String), HashMap<String, Vec<String>>>,
    ocpn: &OCPN,
) -> HashSet<String> {
    //TODO: only use bindings with this event id??? => better efficency
    //TODO: preset must contain whole event?



    //get Binding sequence and used objects
    let (mut binding_sequence, used_objects) = get_binding_sequence_and_used_obj(preset, bindings);

    //queue of states: queue of <Place id, [object_ids]>
    let mut state_queue: VecDeque<Marking> = VecDeque::new();

    //create initial state and push it to the queue
    let mut initial_state = Marking::new();
    //calculate initial Marking
    initial_state = get_initial_marking(ocpn, event_id, &binding_sequence, ocel);


    state_queue.push_back(initial_state);

    let mut result: HashSet<String> = HashSet::new(); //result: list of activities (transition ids)

    while !state_queue.is_empty() {
        let mut current_state = state_queue.pop_front().unwrap();

        //if binding sequence is fully replayed in the current state: no bindinng is left in the sequence
        if binding_sequence.is_empty() {
            //add enabled transitions to result
            result.extend(ocpn.get_enabled_transitions_from_marking(&current_state));
            //TODO: check function: not containing silent transition??? ??
            //TODO: efficeincy could be better maybe: already calculated enabled transitions are calculated again
        }
        if let Some(next_binding) = ocpn.remove_next_binding(&mut binding_sequence, &current_state) { // if binding is fully replayed => no next binding could be enabled
            // next binding is not empty, so we can try to replay it
            //TODO: is checked: IMPORTANT: some transitions need both obj some only one !!!!!
            //TODO: get_next_binding is probably not optimal !!!!!
            
            //execute next binding:
            let state = ocpn.execute_binding(&mut next_binding.clone(), &current_state);
            
            //enqueue resulting state
            state_queue.push_back(state);
            
        }else { //consideration of silent transitions:
            
            let enabled_silent_bindings = ocpn.get_enabled_silent_transition_bindings(&current_state);

            //execute all enabled silent transitions
            for enabled_binding in enabled_silent_bindings {
                //execute binding
                let state = ocpn.execute_binding(&mut enabled_binding.clone(), &current_state);

                //enqueue resulting state
                state_queue.push_back(state);
            }
        }
    }
    result
}





//TODO: FALSE and not used?
//true if: all bindings are possible in the curretn state: i.e. all objects from all bindings are in the right places
fn is_binding_fully_replayed(
    binding_sequence: &HashMap<String, Vec<Vec<String>>>,
    state: &Marking,
) -> bool {
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

pub fn get_binding_sequence_and_used_obj(
    preset: &Vec<OCELEvent>,
    bindings: &HashMap<(String, String), HashMap<String, Vec<String>>>,
) -> (HashMap<String, Vec<Vec<String>>>, HashSet<String>) {
    //not a ordered sequence

    //TODO: what are only visible transitions?
    //<event_type, [[object_ids (for first event with this event_type)], [object_ids (for second event with this event_type), ...]]>
    let mut binding_sequence: HashMap<String, Vec<Vec<String>>> = HashMap::new();
    //used objects
    let mut used_objects: HashSet<String> = HashSet::new();
    for event in preset {
        //push binding of event
        let binding = bindings
            .get(&(event.id.clone(), event.event_type.clone()))
            .unwrap();
        binding_sequence
            .entry(event.event_type.clone())
            .or_insert_with(Vec::new)
            .push({
                let values: Vec<String> = binding.values().flatten().cloned().collect();
                used_objects.extend(values.clone());
                values
            });
    }

    (binding_sequence, used_objects)
}
