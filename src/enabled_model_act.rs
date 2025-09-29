use crate::structs_for_ocpm::{hash_marking, hash_markings, print_marking, Marking, OCPN};
use counter::Counter;
use process_mining::ocel::ocel_struct::OCELEvent;
use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt::format;
use std::fs::File;
use std::io::Write;
use std::task::Context;
use process_mining::OCEL;
use rayon::prelude::*;
use crate::{ NUMBER_OF_ITERATIONS};
use crate::utils::{equals};
use dashmap::DashMap;
use std::sync::{Arc, RwLock};
use std::sync::atomic::{AtomicBool, AtomicU16, Ordering};
use process_mining::event_log::Event;
//for optimization
use rustc_hash::{FxBuildHasher, FxHashMap};
use rustc_hash::FxHashSet;
use nohash_hasher::BuildNoHashHasher;
use crate::MDFA::{abstract_binding_sequnece_with_map, abstract_marking_map, abstract_marking_with_map, compare_marking_multiplicity, get_all_rename_maps, State, MDFA};

static MULTITHREADING: bool = true; //if true, use multithreading for enabled model activities calculation

pub fn get_enabled_model_activities_(
    ocpn: &OCPN,
    bindings: &HashMap<String, Vec<(String, HashMap<String,HashSet<String>>)>>,
    contexts: &HashMap<String, HashMap<String, Counter<Vec<String>>>>,
    context_event_map: &HashMap<u64, HashSet<String>>, //hashed context, event_ids with this context
    all_object_types: &HashSet<String>,
    id_to_event: &HashMap<String, OCELEvent>,
    id_presets: &HashMap<String, Vec<String>>, //event_id, events in presets
) -> FxHashMap<String, FxHashSet<String>> { //event_id, enabled_model_activities

    //println!("Contexts map: {:?}", contexts_map);


    //DEBUG
    //check if there is any event present in 2 differnet contexts
    let mut event_context_check: HashMap<String, u64> = HashMap::new(); //event_id, context hash
    for (context_hash, events) in context_event_map.iter() {
        for event in events {
            if let Some(existing_context) = event_context_check.get(event) {
                if *existing_context != *context_hash {
                    println!("Event {} is present in multiple contexts: {} and {}", event, existing_context, context_hash);
                }else{
                    println!("Event {} is already present in multiple contexts: {}", event, context_hash);
                }
            } else {
                event_context_check.insert(event.clone(), *context_hash);
            }
        }
    }
    let mut DEBUG_event_list: RwLock<HashSet<String>> = RwLock::new(HashSet::new());

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
    let mut prev_replayed_event_map = Arc::new(DashMap::<String, Option<Vec<Marking>>>::with_shard_amount(128)); //event_id, Vec<Marking>
    
    //MDFA approach
    let mut mdfa:MDFA = MDFA::new(ocpn.object_to_type.clone());
    mdfa.add_state(Some("0".to_string()), true); //initial state
    
    //for DEBUG
    if MULTITHREADING {

        let mut results: FxHashMap<String, FxHashSet<String>> = FxHashMap::with_capacity_and_hasher(id_to_event.len(), FxBuildHasher::default());
        let en_act_for_context: Arc<DashMap<u64, FxHashSet<String>>> = Arc::new(DashMap::with_capacity(context_event_map.len())); //HAsh of context, enabled model activities for this context
        
        //stats
        let finished_events: Arc<std::sync::atomic::AtomicUsize> = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let print_lock = Arc::new(AtomicBool::new(false));
        //TODO last 0.2%take very long => maybe use batching?
        let mut context_vec: Vec<_> = context_event_map.iter().collect();
        //sort by number of events in context
        //context_vec.sort_by_key(|(_, events)| -(events.len() as isize)); // TODO add "-" if needed
        //sort by legth of prefix of each event of each context
        context_vec.sort_by_key(|(_, events)| ( // TODO add "-" if needed
            events.iter().map(|event| bindings.get(event).unwrap().len()).sum::<usize>()
                as isize)); //first: context with most events
        //println!("{},{},{},{},{}, {}", context_vec[0].1.iter().map(|e| bindings.get(e).unwrap().len()).sum::<usize>(), context_vec[1].1.iter().map(|e| bindings.get(e).unwrap().len()).sum::<usize>(), context_vec[2].1.iter().map(|e| bindings.get(e).unwrap().len()).sum::<usize>(), context_vec[3].1.iter().map(|e| bindings.get(e).unwrap().len()).sum::<usize>(), context_vec[4].1.iter().map(|e| bindings.get(e).unwrap().len()).sum::<usize>(), context_vec.last().unwrap().1.iter().map(|e| bindings.get(e).unwrap().len()).sum::<usize>());

        println!("Calc enabled model activities for {} contexts", context_vec.len());

        //sort by prefix length
        //context_vec.sort_by_key(|(_, events)|
        //    -(events.iter().map(|event| id_presets.get(event).unwrap().len()).sum::<usize>() as isize) //first: context with largest combined prefix length
        //);
        //let temp = context_vec.pop().unwrap();
        //context_vec.insert(0, temp); //move first element to end of vec => kontext for first events should not be last in queue
        


        //test
        let progress = Arc::new(AtomicU16::new(0));
        let nested_pool = rayon::ThreadPoolBuilder::new().num_threads(9).build().unwrap();

        /*
            (
            events.iter().map(|event| id_presets.get(event).unwrap().len()
            ).sum::<usize>() as isize)
         */


        //context_vec.par_chunks(15).for_each(|chunk| {
            //chunk.iter().for_each(|(ctxt, events)| {
        context_vec.par_iter().for_each(|(ctxt, events)| {


                #[cfg(feature = "progress-percentage")]{

                    //print percentage every 10 iterations
                    if print_lock
                        .compare_exchange(false, true, Ordering::Relaxed, Ordering::Relaxed)
                        .is_ok()
                    {
                        let finished = finished_events.load(Ordering::Relaxed).clone();
                        if finished % 10 == 0 {
                            let percentage = (finished as f64 / id_to_event.len() as f64 * 100.0);
                            progress.store((percentage.round() as usize).try_into().unwrap(), Ordering::Relaxed);
                            println!("Enabled Model act: {:.2}%",
                                     percentage
                            );
                        }
                        //free print lock
                        print_lock.store(false, Ordering::Relaxed);
                    }
                }
                let mut result: FxHashSet<String> = FxHashSet::default();
                //TODO: check if nested parallelism is possible here

                //test nested threading
                //use nested threading if other threads are finished (often happens after 99%)
                if false && progress.load(Ordering::Relaxed) >= 99 && false{

                    let _result: Vec<FxHashSet<String>> = nested_pool.install(||{
                        events
                            .par_iter()
                            .map(|event_id| {
                                //println!("starting event: {}", event_id);

                                //let temp =
                                    get_enabled_model_activities_for_event(
                                    event_id,
                                    contexts.get(event_id).unwrap(),
                                    bindings.get(event_id).unwrap(),
                                    ocpn,
                                    all_object_types,
                                    id_to_event,
                                    //previsous_event_map.get(event_id).unwrap().clone(),
                                    //&mut prev_replayed_event_map,
                                    //&local_copy
                                    &mdfa,
                                )
                                    //;

                                //println!("finished event: {}", event_id);
                                //temp
                            })
                            .collect()
                    });
                    for res in _result{
                        result.extend(res);
                    }

                }else{
                    for event_id in *events {
                        //println!("{}", event_id);
                        //calculate enabled model activities for this event
                        /*
                        let mut prev_replayed_event_map = Arc::clone(&prev_replayed_event_map);
                        let local_copy: HashMap<_, _> = prev_replayed_event_map
                            .iter()
                            .map(|r| (r.key().clone(), r.value().clone()))
                            .collect();

                         */

                        if DEBUG_event_list.read().unwrap().contains(&event_id.clone()) {
                            println!(" ->->->->-> Event {} calculated twice!!!!!!", event_id);
                        }else{
                            DEBUG_event_list.write().unwrap().insert(event_id.clone());
                        }

                        result.extend(get_enabled_model_activities_for_event(
                            event_id,
                            contexts.get(event_id).unwrap(),
                            bindings.get(event_id).unwrap(),
                            ocpn,
                            all_object_types,
                            id_to_event,
                            //previsous_event_map.get(event_id).unwrap().clone(),
                            //&mut prev_replayed_event_map,
                            //&local_copy
                            &mdfa,
                        ));
                    }
                }

                //maybe insert later in single thread?
                //add result to enabled activities for this context
                en_act_for_context.insert(
                    **ctxt,
                    result,
                );


                #[cfg(feature = "progress-percentage")]{
                    finished_events.fetch_add(events.len(), std::sync::atomic::Ordering::Relaxed);
                }


            });
        //});



        #[cfg(feature = "progress-percentage")]{
            print!("Enabled Model act: 100.00%\n");
        }
        //insert enabled activities for each context into results
        for (cntxt, events) in context_event_map.iter() {
            for event in events{
                results.insert(event.clone(), en_act_for_context.get(cntxt).unwrap().clone());
            }
        }
        //println!("mdfa: {:#?}", mdfa.states);
        DEBUG_event_list.write().unwrap().clear();
        println!("CLEAR");

        //save eog to vis.fot NEVER DO THIS!
        //mdfa.export_dot_to_file("/Users/maxbaumeister/RustroverProjects/Fitness-and-Precision-for-OCPM/src/test/MDFA_vis.dot".to_string());




        results
    }else{
        //OLD VERSION:
        //iterate over each different context
        let mut results: FxHashMap<String, FxHashSet<String>> = FxHashMap::default();
        
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
                let mut result: FxHashSet<String> = FxHashSet::default();
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
                        //previsous_event_map.get(event_id).unwrap().clone(),
                        //&mut prev_replayed_event_map,
                        //&local_copy
                        &mdfa,
                    ));
                }
    
                //TODO parallel insert and ArcHashSet
                //add result to results
                for event_id in events {
                    results.insert(event_id.clone(), result.clone());
                }
            } else {

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
    //previous_event: String,
    //prev_replayed_event_map: &mut Arc<DashMap<String, Option<Vec<Marking>>>>,
    //local_map: &HashMap<String, Option<Vec<Marking>>>, //local map of prev_replayed_event_map to avoid borrowing issues
    mdfa_: &MDFA,
    
) -> FxHashSet<String> {

    //println!("Event: {}", event_id);
    let mut binding = binding_.clone(); //clone binding to avoid borrowing issues
    let original_binding_len = binding.len();



    //DEBUG
    //if !vec!["7498".to_string(), ].contains(event_id) { return HashSet::new(); }
    //let event = id_to_event.get(event_id).unwrap();



    //--------------------------------------------
    //replay binding sequence:
    //queue of states: queue of (Marking: <Place id, [object_ids]>, index of binding sequence, Hash of marking)
    //let mut state_queue: VecDeque<(Marking, Vec<(String, HashMap<String, HashSet<String>>)>, u64)> = VecDeque::new();
    let mut state_queue: VecDeque<(Marking, usize)> = VecDeque::new();


    //get needed objs
    let mut needed_objects: HashMap<String,HashSet<String>> = HashMap::new(); //object_type: objects

    /*
    for binding__ in binding.iter() {
        if binding__.0.to_lowercase().contains("goods"){
            println!("Binding 'goods'");
        }
    }

     */

    //get all associated objects
    for object_type in all_object_types {
        for (_, objs) in binding.iter() {
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




        let initial_marking = get_initial_marking(
            event_id,
            all_object_types,
            &binding,
            context,
            id_to_event,
            &needed_objects,
            ocpn,
        );

        state_queue.push_back((initial_marking.clone(), 0)); //TODO: clone just for debug
    


    //REPLAY -------

    /*
    MDFA:
     - to complex to start in not initial places?
     */

    //MDFA:
    //check if initial marking is already in MDFA
    //TODO: quick: cloned fix for thread safety, could be optimized later
    //TODO: could also try to start somewhere in MDFA, but could lead to extreme memory usage
    let mdfa = mdfa_;
    //try each initial marking
    let initial_state = mdfa.initial_states.read().unwrap().clone();
    //DEBUG
    if true{
        for initial_state_mdfa in initial_state.iter() {
            let initial_state = mdfa.states.get(initial_state_mdfa).unwrap().read().unwrap().clone();


            //check if there is a fitting marking in this initial state
            let initial_markings = initial_state.markings.clone();
            for marking_init_ in initial_markings.values() {

                //TODO: ASSUMPTION: initial markings are always Vecs with only one element!!!
                let marking_init = marking_init_.get(0).unwrap();

                if !compare_marking_multiplicity(marking_init, &initial_marking, &ocpn.object_to_type) {
                    //=> Marking can't be abstracted to initial marking
                    continue;
                }


                //get all rename maps
                let rename_maps = get_all_rename_maps(&initial_marking, &ocpn.object_to_type);
                for map in rename_maps {
                    //try if this one fits
                    let abst_marking = abstract_marking_with_map(&initial_marking, &map);

                    //check if marking is equal to initial marking of the mdfa
                    if marking_init != &abst_marking { continue; }

                    //abstract binding
                    let abstraceted_binding_seq = abstract_binding_sequnece_with_map(&binding, &map);

                    //try to replay binding on mdfa
                    let (state, binding_seq, markings) = mdfa.replay_next_state(
                        initial_state,
                        &vec![marking_init.clone()],
                        &abstraceted_binding_seq,
                    );

                    // if binding sequence is empty, we can directly return the enabled activities
                    if binding_seq.is_empty() {
                        //get enabled transitions from state

                        let hash = &hash_markings(&markings, &ocpn.object_to_type);
                        let mut ret = state.enabled_activities.get(hash)
                            .unwrap_or(&FxHashSet::default())
                            .clone();
                        //println!("full mdfa: {:?} ; event: {:?}", ret, event_id);

                        if ret.is_empty(){
                            //for this Marking the enabled activities are not calculated yet!!!
                            let enabled_activities: HashSet<String, FxBuildHasher> = markings
                                .iter()
                                .flat_map(|marking| {
                                    ocpn.get_enabled_transitions_from_marking(marking)
                                        .into_iter()
                                        .filter_map(|trans| ocpn.transitions.get(&trans).map(|t| t.activity.clone()))
                                })
                                .collect();
                            //println!("Enabled enabled transitions from {:?}", enabled_activities);
                            mdfa.states.get_mut(&state.id).unwrap().write().unwrap().enabled_activities.insert(hash.clone(), enabled_activities.clone());
                            ret = enabled_activities;
                        }


                        return ret;


                    }else{
                        //binding cound not be fully replayed, contine replaying on ocpn
                        state_queue.clear();
                        state_queue.extend(
                            markings.into_iter().map(|m| (m, original_binding_len - binding_seq.len())) //insert the markings of the current state into the queue
                        );
                        //println!("parmeters: event_id: {:?}, state_queue: {:?}, binding: {:?}, original_binding_len: {}", event_id, state_queue, binding_seq, original_binding_len );

                        let ret = replay_marking(ocpn, event_id, &mut state_queue, &mut binding_seq.clone(), original_binding_len, &mdfa, &Some(state));
                        //println!("partial mdfa {:?} ; event:{:?}", ret, event_id);
                        return ret;
                    }



                }
            }
        }
    }
    //stateu queue can only have the initial marking
    //every object in the binding sequence must be in the initial marking
    let (new_initial_marking, rename_map) = abstract_marking_map(&state_queue.pop_front().unwrap().0, &ocpn.object_to_type);
    state_queue.push_back((new_initial_marking, 0));

    for (_, ot_objs) in binding.iter_mut() {
        for (_, objs) in ot_objs.iter_mut() {
            let mut renamed_objs = HashSet::new();
            for obj in objs.iter() {
                if let Some(new_obj) = rename_map.get(obj) {
                    renamed_objs.insert(new_obj.clone());
                }
            }
            *objs = renamed_objs;
        }
    }
    //println!("parameters: event_id: {:?}, state_queue: {:?}, binding: {:?}, original_binding_len: {}", event_id, state_queue, binding, original_binding_len );
    let ret = replay_marking(ocpn, event_id, &mut state_queue, &mut binding, original_binding_len, &mdfa, &None);
    //println!("normal: {:?} ; event: {:?}", ret, event_id);
    ret
}

fn replay_marking(
    ocpn: &OCPN, 
    event_id: &String, 
    state_queue: &mut VecDeque<(Marking, usize)>, 
    binding: &mut Vec<(String, HashMap<String, HashSet<String>>)>,
    original_binding_len: usize,
    mdfa: &MDFA,
    start_state: &Option<State>,

) -> FxHashSet<String> {



    //mdfa:
    //save initial markings to mdfa

    //DEBUG
    let mut ITER_COUNT: usize = 0;

    //mdfa
    let mut next_binding_queue: VecDeque<(Marking, usize)> = VecDeque::new();


    let mut result: FxHashSet<String> = FxHashSet::default(); //result: list of activities
    // u64 is hashed => NoHashHaser could be used
    //let mut seen_markings: HashSet<(u64, u32)> = HashSet::new(); //Hash of Marking, len of binding
    let mut seen_markings: HashSet<u64, BuildNoHashHasher<u64>> = HashSet::with_hasher(BuildNoHashHasher::default());

    //let mut seen_markings: FastSeenMarkingHash = HashSet::default();
    //TODO is same for all events, so can be moved outside of function
    //let mut cache_enabled_silent_bindings: HashMap<u64, Vec<(String, HashMap<String,HashSet<String>>)>> = HashMap::new(); //cache for enabled silent bindings: <Hash of marking, vec of bindings>
    let maximal_queue_size = 5_000_000;

    //TODO faster do declar before loop
    let mut temp: (Marking, usize);
    let mut current_state: Marking;
    let mut current_binding: Vec<(String, HashMap<String, HashSet<String>>)>;

    //MDFA
    let mut last_markings: Vec<Marking> = vec![];
    last_markings.extend(state_queue.iter().map(|(m, _)| m.clone()));


    let mut current_mdfa_state = start_state.clone().unwrap_or_else(||{
        let id = mdfa.initial_states.read().unwrap().iter().next().unwrap().clone();
        mdfa.states.get(&id).unwrap().read().unwrap().clone()
    });

    //insert
    let mut hashed_last_markings = hash_markings(&last_markings, &ocpn.object_to_type);
    mdfa.states.get_mut(&current_mdfa_state.id).unwrap().write().unwrap().markings.insert(hashed_last_markings, last_markings.clone());
    last_markings.clear();

    //let mut current_binding = binding_.clone(); //current binding sequence to replay
    while !state_queue.is_empty() || !next_binding_queue.is_empty() {

        if state_queue.len() >= 1_000_000 {
            println!("ahhhha");
            state_queue.clear();
        }

        if state_queue.is_empty() {
            state_queue.extend(next_binding_queue.clone());
            next_binding_queue.clear();

            //save markings in mdfa:
            hashed_last_markings = hash_markings(&last_markings, &ocpn.object_to_type);
            mdfa.states.get(&current_mdfa_state.id).unwrap().write().unwrap().markings.insert(hashed_last_markings, last_markings.clone());
            last_markings.clear();

        }

        //println!("{}", state_queue.len());

        //TODO !!!!! IF BINDING SEQUENCE IS 0 ALL OTHER STATES CAN BE DELETED??? => cant be the richt states ig => is false i think!!!
        //println!("{}", state_queue.len());
        if state_queue.len() > maximal_queue_size { println!("ABORT! statequeu length for event: {}", event_id); //panic!()? this lenth sould not be reached!
            break;}



        temp = state_queue.pop_front().unwrap();
        //println!("temp: {:?} for event:{:?}", temp.clone(), event_id);
        current_state = temp.0;
        current_binding = binding[temp.1.clone()..].to_vec();
        //println!("current_binding: {:?} for event:{}", current_binding, event_id);


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
        if let Some(next_binding) = ocpn.remove_next_binding_(&mut current_binding, &current_state) {//if a binding is enabled, remove it
            // there is a binding enabled
            //DEBUG
            //execute next binding:
            let next_state = ocpn.execute_binding_(&next_binding, &current_state);
            //enqueue resulting state
            //only push if state new or binding len is different
            let hashed_next_state = hash_marking(&next_state, &ocpn.object_to_type, &Some(current_binding.len() as u32));
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

            //DEBUG_states.push((next_state.clone(), current_binding.clone()));
            seen_markings.insert(hashed_next_state);
            next_binding_queue.push_back((next_state.clone(), original_binding_len - current_binding.len()));

            //mdfa: save new markings in right state
            let to_state_id:Option<String> = mdfa.states.get(&current_mdfa_state.id.clone())
                .unwrap().read().unwrap().
                next_state.read().unwrap()
                .get(&next_binding.0).cloned();
            let to_state = mdfa.add_state(to_state_id.clone(), false).read().unwrap().clone();
            mdfa.add_arc(Some(current_mdfa_state.id.clone()), next_binding.0, Some(to_state.id.clone()));

            current_mdfa_state = to_state;

            //TODO Markings are not added!!!
            last_markings.push(next_state);

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
            for (trans, enabled_comb_binding) in enabled_silent_bindings {
                //DEBUG / TEST /
                if enabled_comb_binding.len() > 1 {
                    panic!("To many object types")
                }
                // one binding can have multiple objects => iterate over them
                // should only be one object type, but can have multiple objects
                for (object_type, objects) in enabled_comb_binding.iter() {
                    for object in objects {
                        let mut enabled_binding = (trans.clone(), HashMap::from([(object_type.clone(), HashSet::from([object.clone()]))]));

                        //execute binding
                        let state = ocpn.execute_binding_(&mut enabled_binding, &current_state); //TODO check func

                        //enqueue resulting state
                        //only push if state new or binding len is different
                        let hashed_next_state = hash_marking(&state, &ocpn.object_to_type, &Some(current_binding.len() as u32));


                        if seen_markings.contains(&hashed_next_state.clone()){
                            //print!(".");
                            continue;
                        }
                        //println!("------- \n \n -------");
                        //println!("CON: ");
                        //print_marking(&DEBUG_states.iter().map(|(s, _)| s).collect::<Vec<&Marking>>());
                        //println!("\nIN: {:?}", state);

                        //DEBUG_states.push((state.clone(), current_binding.clone()));
                        seen_markings.insert(hashed_next_state);
                        state_queue.push_back((state, temp.1));
                    }
                }
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

    drop(seen_markings);
    //mdfa
    mdfa.states.get_mut(&current_mdfa_state.id).unwrap().write().unwrap().enabled_activities.insert(hashed_last_markings, result.clone());
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

