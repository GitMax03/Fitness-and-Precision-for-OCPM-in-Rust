use crate::structs_for_ocpm::{Marking, OCPN};
use counter::Counter;
use process_mining::ocel::ocel_struct::OCELEvent;
use std::collections::{HashMap, HashSet, VecDeque};
use std::task::Context;
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
    
    
    //TODO: quick fix change result from transition_id to activity
    result = result.iter()
        .map(|(event, enabled_trans)| {
            (event.clone(), enabled_trans.iter()
                .map(|transition_id| ocpn.transitions.get(transition_id)
                    .or(ocpn.silent_transtions.get(transition_id))
                    .unwrap().activity.clone())
                .collect::<HashSet<String>>())
        })
        .collect();

    result
}


///ocpn must have initial marking set!!!
pub fn get_ebabled_model_activities_for_event(
    event_id: &String,
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
    let initial_state = ocpn.initial_marking.clone().unwrap();
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
        }//TODO: deuque from binding sequence => gets faster
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
                //execute binding TODO: seems tau is executet twice
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
