use crate::structs_for_ocpm::{hash_marking, print_marking, Marking, OCPN};
use counter::Counter;
use process_mining::ocel::ocel_struct::OCELEvent;
use std::collections::{HashMap, HashSet, VecDeque};
use std::task::Context;
use process_mining::OCEL;
use rayon::prelude::*;
use crate::NUMBER_OF_ITERATIONS;
use crate::utils::{equals};
use dashmap::DashMap;
use std::sync::{Arc, RwLock};
use std::sync::atomic::{AtomicBool, Ordering};
use process_mining::event_log::Event;

static MULTITHREADING: bool = true; //if true, use multithreading for enabled model activities calculation

pub fn get_enabled_model_activities_(
    ocpn: &OCPN,
    bindings: &HashMap<String, Vec<(String, HashMap<String,HashSet<String>>)>>,
    contexts: &HashMap<String, HashMap<String, Counter<Vec<String>>>>,
    context_event_map: &HashMap<u64, HashSet<String>>, //hashed context, event_ids with this context
    all_object_types: &HashSet<String>,
    id_to_event: &HashMap<String, OCELEvent>,
    id_presets: &HashMap<String, Vec<String>>, //event_id, events in presets
) -> HashMap<String, HashSet<String>> { //event_id, enabled_model_activities

    //println!("Contexts map: {:?}", contexts_map);


    //TODO: idear:
    /*
    if binding sequence is fully replayed, and statequeu ist very long,
    => save all states for following event
    => next event in this case will not have to replay binding sequence again

     */
    //create map of each previsous event for each event_id
    let mut previsous_event_map: HashMap<String, String> = HashMap::new();
    for (event_id, preset_events) in id_presets.iter() {
        previsous_event_map.entry(event_id.clone()).or_insert_with(|| preset_events.last().unwrap_or(&"".to_string()).clone());
    }
    //map of event ids and Markings of all states that replayed the binding sequence of the event
    //let mut prev_replayed_event_map: DashMap<String, RwLock<Vec<HashMap<String, HashSet<String>>>>> = DashMap::new(); //event_id, Vec<Marking>
    let mut prev_replayed_event_map = Arc::new(DashMap::<String, Option<Vec<Marking>>>::with_shard_amount(512)); //event_id, Vec<Marking>
    
    //for DEBUG
    if MULTITHREADING {

        let mut results: HashMap<String, HashSet<String>> = HashMap::new();
        let mut en_act_for_context: Arc<DashMap<u64, HashSet<String>>> = Arc::new(DashMap::new()); //HAsh of context, enabled model activities for this context
        
        //stats
        let finished_events: Arc<std::sync::atomic::AtomicUsize> = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let print_lock = Arc::new(AtomicBool::new(false));
        
        context_event_map.par_iter().for_each(|(cntxt, events)| {
            #[cfg(feature = "progress-percentage")]{

                //print percentage every 5 iterations
                if print_lock
                    .compare_exchange(false, true, Ordering::Relaxed, Ordering::Relaxed)
                    .is_ok()
                {
                    let finished = finished_events.load(Ordering::Relaxed).clone();
                    if finished % 5 == 0 {
                        println!("Enabled Model act: {:.2}%",
                                 (finished as f64 / id_to_event.len() as f64 * 100.0)
                        );
                    }
                    //free print lock
                    print_lock.store(false, Ordering::Relaxed);
                }
            }
            
            

            let mut result: HashSet<String> = HashSet::new();
            for event_id in events {
                //println!("{}", event_id);
                //calculate enabled model activities for this event
                /*
                let mut prev_replayed_event_map = Arc::clone(&prev_replayed_event_map);
                let local_copy: HashMap<_, _> = prev_replayed_event_map
                    .iter()
                    .map(|r| (r.key().clone(), r.value().clone()))
                    .collect();
                    
                 */
                
                result.extend(get_enabled_model_activities_for_event(
                    event_id,
                    contexts.get(event_id).unwrap(),
                    bindings.get(event_id).unwrap(),
                    ocpn,
                    all_object_types,
                    id_to_event,
                    previsous_event_map.get(event_id).unwrap().clone(),
                    //&mut prev_replayed_event_map,
                    //&local_copy
                ));
            }

            //maybe insert later in single thread?
            //add result to enabled activities for this context
            en_act_for_context.insert(
                *cntxt,
                result,
            );


            #[cfg(feature = "progress-percentage")]{
                finished_events.fetch_add(events.len(), std::sync::atomic::Ordering::Relaxed);
            }


        });
        //insert enabled activities for each context into results
        for (cntxt, events) in context_event_map.iter() {
            for event in events{
                results.insert(event.clone(), en_act_for_context.get(cntxt).unwrap().clone());
            }
        }
        
        results
    }else{
        //OLD VERSION:
        //iterate over each different context
        let mut results: HashMap<String, HashSet<String>> = HashMap::new();
        
        for (_, events) in context_event_map {
            #[cfg(feature = "progress-percentage")]{
    
                //print percentage every 5 iterations
                if results.len() % 5 == 0 {
                    println!("Enabled Model act: {}%", (results.len() as f64 / id_to_event.len() as f64 * 100.0 * 100.0).round() / 100.0);
                }
            }
    
            //enabled model activities for this context
            //if: just for debug
            // false => multithread
            if !MULTITHREADING {
                let mut result: HashSet<String> = HashSet::new();
                let local_copy: HashMap<_, _> = prev_replayed_event_map
                    .iter()
                    .map(|r| (r.key().clone(), r.value().clone()))
                    .collect();
                for event_id in events {
                    //println!("{}", event_id);
                    //calculate enabled model activities for this event
                    //TODO use multithreading here!
                    result.extend(get_enabled_model_activities_for_event(
                        event_id,
                        contexts.get(event_id).unwrap(),
                        bindings.get(event_id).unwrap(),
                        ocpn,
                        all_object_types,
                        id_to_event,
                        previsous_event_map.get(event_id).unwrap().clone(),
                        //&mut prev_replayed_event_map,
                        //&local_copy
                    ));
                }
    
                //TODO parallel insert and ArcHashSet
                //add result to results
                for event_id in events {
                    results.insert(event_id.clone(), result.clone());
                }
            } else {
    
                //multithreaded
                let result: HashSet<String> = events
                    .par_iter()
                    .map(|event_id| {
                        let mut prev_replayed_event_map = Arc::clone(&prev_replayed_event_map);
                        let local_copy: HashMap<_, _> = prev_replayed_event_map
                            .iter()
                            .map(|r| (r.key().clone(), r.value().clone()))
                            .collect();
    
                        get_enabled_model_activities_for_event(
                            event_id,
                            contexts.get(event_id).unwrap(),
                            bindings.get(event_id).unwrap(),
                            ocpn,
                            all_object_types,
                            id_to_event,
                            previsous_event_map.get(event_id).unwrap().clone(),
                            //&mut prev_replayed_event_map,
                            //&local_copy
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


    //DEBUG
    /*
    for (_, events) in context_event_map.iter() {
        
        for event_id in events {
            if event_id == "1789" {
                get_enabled_model_activities_for_event(
                    event_id,
                    contexts.get(event_id).unwrap(),
                    bindings.get(event_id).unwrap(),
                    ocpn,
                    all_object_types,
                    id_to_event,
                    id_presets.get(event_id).unwrap_or(&vec![]).last().unwrap_or(&"".to_string()).clone(),
                    &mut prev_replayed_event_map,
                );
            }
        }
    }
    return results;
    */
}

pub fn get_enabled_model_activities_for_event(
    event_id: &String,
    context: &HashMap<String, Counter<Vec<String>>>, //context of event_id: object type: counter[activities]
    binding_: &Vec<(String, HashMap<String, HashSet<String>>)>, //map of binding sequences: activity, HashMap< object type: [objects]>
    ocpn: &OCPN,
    all_object_types: &HashSet<String>,
    id_to_event: &HashMap<String, OCELEvent>,
    previous_event: String,
    //prev_replayed_event_map: &mut Arc<DashMap<String, Option<Vec<Marking>>>>,
    //local_map: &HashMap<String, Option<Vec<Marking>>>, //local map of prev_replayed_event_map to avoid borrowing issues
    
) -> HashSet<String> {

    let mut binding = binding_.clone(); //clone binding to avoid borrowing issues
    let original_binding_len = binding.len();
    
    

    //DEBUG
    //if !vec!["7498".to_string(), ].contains(event_id) { return HashSet::new(); }
    //let event = id_to_event.get(event_id).unwrap();
    
    
    
    //--------------------------------------------
    //replay binding sequence:
    //queue of states: queue of (Marking: <Place id, [object_ids]>, index of binding sequence, Hash of marking)
    //let mut state_queue: VecDeque<(Marking, Vec<(String, HashMap<String, HashSet<String>>)>, u64)> = VecDeque::new();
    let mut state_queue: VecDeque<(Marking, usize, u64)> = VecDeque::new();


    //get needed objs
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
    //if previos event was already replayed, use the map to get the markings as initial state
    'cache_init_state: {
        //if let Some(markings) = local_map.get(&previous_event) {
        if false{
        
        //problem: sometimes the previous event does not have all needed objects
        //TODO: fix: insert the missing objects into the initial places:
        //TODO: Too much overhead?
        //quickfix: only use when all objects are present in the first marking (if fist marking contains it, all other markings will contain it too)
        
        //test:
        let markings = Some(vec![Marking::new(); 2]);
            
        //let mut extend_marking: HashMap<String, HashSet<String>> = HashMap::new(); //place_id: objects
        let first_marking = markings.clone().unwrap().first().unwrap().clone();
        //check first marking for needed objects
        for obj in needed_objects.values().flat_map(|objs| objs) {
            if !first_marking.values().any(|v| v.contains(obj)) {
                //TODO quickfix:
                let initial_marking = get_initial_marking(
                    event_id,
                    all_object_types,
                    &binding,
                    context,
                    id_to_event,
                    &needed_objects,
                    ocpn,
                );

                state_queue.push_back((initial_marking.clone(), 0, hash_marking(&initial_marking.clone(), &ocpn.object_to_type))); //TODO: clone just for debug
                //println!("we breaking free... saaaawing ..... flieeeeeing");
                break 'cache_init_state; //break if statement
            }
        }
        
        for marking in markings.clone().unwrap().iter() {
            //add marking to state queue
            state_queue.push_back((marking.clone(), original_binding_len -1 , hash_marking(&marking, &ocpn.object_to_type)));
        }
        //println!("Using previous replayed markings for event: {}; skipped binding seq. len: {}", event_id, original_binding_len - 1);
        //println!("State queue: {:?}\n", state_queue);
        
        
    }else{

        let initial_marking = get_initial_marking(
            event_id,
            all_object_types,
            &binding,
            context,
            id_to_event,
            &needed_objects,
            ocpn,
        );

        state_queue.push_back((initial_marking.clone(), 0, hash_marking(&initial_marking.clone(), &ocpn.object_to_type))); //TODO: clone just for debug

    }}
    let mut prev_replayed_event_map_: Vec<Marking> = vec![];
    

    //REPLAY -------
    

    let mut result: HashSet<String> = HashSet::new(); //result: list of activities
    //let mut seen_markings: HashMap<u64, Vec<(String, HashMap<String, HashSet<String>>)>> = HashMap::new(); //seen amrkings: <Hash of marking, len of binding> to avoid infinite loops
    let mut seen_markings: HashSet<(u64, usize)> = HashSet::new();
    //TODO is same for all events, so can be moved outside of function
    let mut cache_enabled_silent_bindings: HashMap<u64, Vec<(String, HashMap<String,HashSet<String>>)>> = HashMap::new(); //cache for enabled silent bindings: <Hash of marking, vec of bindings>
    let maximal_queue_size = ocpn.places.len().pow(10) * 10;
    //let mut current_binding = binding_.clone(); //current binding sequence to replay
    while !state_queue.is_empty() {

        //println!("{}", state_queue.len());

        //TODO !!!!! IF BINDING SEQUENCE IS 0 ALL OTHER STATES CAN BE DELETED??? => cant be the richt states ig => is false i think!!!
        //println!("{}", state_queue.len());
        if state_queue.len() > maximal_queue_size { println!("ABORT! statequeu length for event: {}", event_id); //panic!()? this lenth sould not be reached!
            break;}

        let temp = state_queue.pop_front().unwrap();
        let current_state = temp.0;
        let mut current_binding = binding[temp.1.clone()..].to_vec();
        let hashed_current_state = temp.2;
        

        //DEBUG
        /*
        let marking = HashMap::from([
            ("offer5".to_string(), HashSet::from(["Offer_212771240".to_string()])),
            ("offer1".to_string(), HashSet::from(["Offer_1933060811".to_string()])),
            ("application2".to_string(), HashSet::from(["Application_1971576090".to_string()])),
            ("application25".to_string(), HashSet::from(["Application_1971576090".to_string()])),
        ]);
        if current_state == marking {
            println!("Found current state: {:?}", current_state);
        }

         */

        //if binding sequence is fully replayed in the current state: no bindinng is left in the sequence
        if current_binding.is_empty() {

            if state_queue.is_empty() {
                //println!("state queue empthy event: {}", event_id);
            }

            //if binding sequence is fully replayed in this state add it to the prev_replayed_event_map_
            //prev_replayed_event_map_.push(current_state.clone());

            //add enabled transitions to result
              result.extend(ocpn.get_enabled_transitions_from_marking(&current_state)
                .into_iter()
                .map(|t_id| ocpn.transitions.get(&t_id).unwrap().activity.clone()));
            //println!("Found {} enabled transitions for event: {}", result.len(), event_id);
        }
        //TODO: idea: if binding not fully replayed -> only move tokens that are nessary  => false?: solent transitions with two inputs?
        //get next enabled binding
        if let Some(mut next_binding) = ocpn.remove_next_binding_(&mut current_binding, &current_state) {//if a binding is enabled, remove it
            // there is a binding enabled
        //DEBUG
            //execute next binding:
            let next_state = ocpn.execute_binding_(&mut next_binding, &current_state);
            //enqueue resulting state
            //only push if state new or binding len is different
            let hashed_next_state = hash_marking(&next_state, &ocpn.object_to_type);
            //check if state is already seen is not needed, because binding is different


            //println!("------- \n \n -------");
            //println!("CON: ");
            //print_marking(&DEBUG_states.iter().map(|(s, _)| s).collect::<Vec<&Marking>>());
            //println!("\nIN: {:?}", next_state);

            //if current_binding.len() == 1 {
                //binding fully replayed, clear queue to calc result
                //all states are possible traces to replay binding seq. if binding seq. is fully relayed, all other states i.e. traces are false/unessasry
                //nessasry: current marking = marking directly after binding sequence => go throuh binding sequence backwards and restore all objects (if not already restored)
                // => clar seen_markings and start looking for enabled transitions again TODO
                //not applicable => e.g. 2 traces lead to activits, one creates additional object...
                //state_queue.clear();
                
                
                //DEBUG define marking by hand
                /*
                let marking = HashMap::from([
                    ("offer5".to_string(), HashSet::from(["Offer_212771240".to_string()])),
                    ("offer1".to_string(), HashSet::from(["Offer_1933060811".to_string()])),
                    ("application2".to_string(), HashSet::from(["Application_1971576090".to_string()])),
                    ("application25".to_string(), HashSet::from(["Application_1971576090".to_string()])),
                ]);

                 */
                //state_queue.push_back((marking.clone(), current_binding.clone(), hash_marking(&marking)));
                
            //}
            let marking = Marking::from([
                ("offer5".to_string(), HashSet::from(["Offer_212771240".to_string()])),
                ("offer1".to_string(), HashSet::from(["Offer_1933060811".to_string()])),
                ("application2".to_string(), HashSet::from(["Application_1971576090".to_string()])),
                ("application25".to_string(), HashSet::from(["Application_1971576090".to_string()])),
            ]);
            if next_state == marking {
                println!("Found state: {:?}", next_state);
            }

            //DEBUG_states.push((next_state.clone(), current_binding.clone()));
            seen_markings.insert((hashed_next_state.clone(), current_binding.len()));
            state_queue.push_back((next_state.clone(), original_binding_len - current_binding.len(), hash_marking(&next_state, &ocpn.object_to_type)));


//SILENT TRANSITIONS -------
        }else{ //there is no enabled binding

            /*
            if
            (current_state.contains_key(&"offer4".to_string()) ||
                current_state.contains_key(&"offer5".to_string())) && 
                //current_state.contains_key(&"application27".to_string()) &&
                current_binding.is_empty() {
                println!("Found fully replayed binding for event: {}", event_id);
            }
            
             */
            
            let mut enabled_silent_bindings:Vec<(String, HashMap<String, HashSet<String>>)> = vec![];
            //check for any enabled silent transitions
            if false{
                /* for caching: uses too much memory
            if let Some(cached) = cache_enabled_silent_bindings.get(&hashed_current_state) {

                //println!("Found cached silent bindings for state: {}", hashed_current_state);
                //get enabled silent transitions from cache
                enabled_silent_bindings = cached.clone();
            }

                 */
            } else{
                enabled_silent_bindings = ocpn.get_enabled_silent_transitions_bindings_(&current_state);
                //cache_enabled_silent_bindings.insert(hashed_current_state, (enabled_silent_bindings.clone()));
            }
            //let enabled_silent_bindings = ocpn.get_enabled_silent_transitions_bindings_(&current_state);
            //let enabled_silent_bindings = ocpn.get_enabled_silent_transition_bindings(&current_state);

            //execute all enabled silent transitions
            for (trans, enabled_binding) in enabled_silent_bindings {
                
                //let mut enabled_binding = (trans.clone(), HashMap::from([(object_type.clone(), HashSet::from([object.clone()]))]));

                //execute binding
                let state = ocpn.execute_binding_(&mut (trans, enabled_binding), &current_state); //TODO check func

                //enqueue resulting state
                //only push if state new or binding len is different
                let hashed_next_state = hash_marking(&state, &ocpn.object_to_type);


                if seen_markings.contains(&(hashed_next_state.clone(), current_binding.len())){
                    //print!(".");
                    continue;
                }
                //println!("------- \n \n -------");
                //println!("CON: ");
                //print_marking(&DEBUG_states.iter().map(|(s, _)| s).collect::<Vec<&Marking>>());
                //println!("\nIN: {:?}", state);

                //DEBUG_states.push((state.clone(), current_binding.clone()));
                seen_markings.insert((hashed_next_state.clone(), current_binding.len()));
                state_queue.push_back((state.clone(), temp.1, hash_marking(&state, &ocpn.object_to_type)));
                
                
            }
        }
    }
    if result.is_empty(){
        println!("No enabled transitions found for event: {}", event_id);
    }
    
    //add replaying states to prev_replayed_event_map
    //if !prev_replayed_event_map.contains_key(event_id) {
    //    prev_replayed_event_map.insert(event_id.clone(), Some(prev_replayed_event_map_));
    //}

    result
}


fn get_initial_marking(
    event_id: &String,
    all_object_types: &HashSet<String>,
    binding: &Vec<(String, HashMap<String, HashSet<String>>)>, //map of binding sequences: activity, HashMap< object type: [objects]>
    context: &HashMap<String, Counter<Vec<String>>>, //context of event_id: object type: counter[activities]
    id_to_event: &HashMap<String, OCELEvent>,
    needed_objects: &HashMap<String, HashSet<String>>, //object_type: objects needed for binding
    ocpn: &OCPN,
) -> Marking{

    


    //add them to initial marking
    let mut initial_marking: Marking = Marking::new();
    let mut object_count = 0;
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
                    object_count += 1;
                }
                //TODO evt. add additional objects to initial markingxq
            }
        }
    }
    
    initial_marking
}

