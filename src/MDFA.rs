//Marking DFA for caching Markings

use crate::structs_for_ocpm::{hash_marking, hash_markings, Marking};
use ahash::{AHashMap, AHashSet, AHasher};
use itertools::Itertools;

use std::collections::{HashMap, HashSet, VecDeque};
use std::fs::File;
use std::io::Write;
use std::sync::{Arc, RwLock};
use dashmap::{DashMap, Entry};
use nohash_hasher::BuildNoHashHasher;
use petgraph::dot::Dot;
use petgraph::graph::DiGraph;
use petgraph::visit::NodeRef;
use rustc_hash::{FxHashMap, FxHashSet};

pub struct MDFA {
    //Marking vecs of initial state are always size = 1!!!
    pub initial_states: RwLock<AHashSet<String>>, // id of initial States of the DFA
    pub states: DashMap<String, Arc<RwLock<State>>>, //id: state

    pub(crate) object_to_type: Arc<HashMap<String, String>>, // Object to type mapping
}
//#[derive(Eq, PartialEq, Clone)]
#[derive(Debug)]
pub struct State {
    pub markings: HashMap<u64, Vec<Marking>, BuildNoHashHasher<u64>>,  //hash of preivos marking(s), Markings
    pub next_state: RwLock<HashMap<String, String>>, // Transitions to next states: label: state id
    pub enabled_activities: HashMap<u64, FxHashSet<String>, BuildNoHashHasher<u64>>, // hash of current (abstracted) makrings: enabled activities in this state (could also use hashed value)
    
    pub id: String,
}

impl State {
    pub fn new(id: String) -> Self {
        State {
            markings: HashMap::with_hasher(BuildNoHashHasher::default()), // Hashed Markings of the state
            next_state: RwLock::new(HashMap::new()), //TODO: better with DashMap?
            enabled_activities: HashMap::with_hasher(BuildNoHashHasher::default()),
            id: id,
        }
    }
}

impl Clone for State{
    fn clone(&self) -> Self {
        State {
            markings: self.markings.clone(),
            next_state: RwLock::new(self.next_state.read().unwrap().clone()),
            enabled_activities: self.enabled_activities.clone(),
            id: self.id.clone(),
        }
    }
}


impl Clone for MDFA{
    fn clone(&self) -> Self {
        MDFA {
            initial_states: RwLock::new(self.initial_states.read().unwrap().clone()),
            states: self.states.clone(),
            object_to_type: Arc::clone(&self.object_to_type),
        }
    }
}

impl MDFA {
    pub fn new(object_to_type: HashMap<String, String>) -> Self {
        MDFA {
            initial_states: RwLock::new(AHashSet::new()),
            states: DashMap::new(),
            object_to_type: Arc::from(object_to_type),
        }
    }



    pub fn add_state(&self, state_id: Option<String>, is_initial: bool) -> Arc<RwLock<State>> {

        //if self.states.contains_key(&state_id) {
        //    return self.states[&state_id].clone();
        //}
        if state_id.is_none() {
            let id = self.states.len().to_string();
            let new_state = Arc::new(RwLock::new(State::new(id.clone())));
            self.states.insert(id, new_state.clone()); 
            return new_state;
        }
       
        match self.states.entry(state_id.clone().unwrap()) {
            Entry::Occupied(e) => e.get().clone(),
            Entry::Vacant(e) => {

                if is_initial {
                    self.initial_states.write().unwrap().insert(state_id.clone().unwrap());
                }

                let new_state = Arc::new(RwLock::new(State::new(state_id.unwrap())));
                e.insert(new_state.clone());
                new_state
            }
        }
    }
    pub fn add_state_with_markings(&self, state_id: String, markings: HashMap<u64, Vec<Marking>, BuildNoHashHasher<u64>>, is_initial: bool) -> Arc<RwLock<State>> {

        //if self.states.contains_key(&state_id) {
        //    return self.states[&state_id].clone();
        //}

        match self.states.entry(state_id.clone()) {
            Entry::Occupied(e) => e.get().clone(),
            Entry::Vacant(e) => {

                if is_initial {
                    self.initial_states.write().unwrap().insert(state_id.clone());
                }

                let new_state = Arc::new(RwLock::new(State::new(state_id)));
                new_state.write().unwrap().markings = markings;
                e.insert(new_state.clone());
                new_state
            }
        }
    }

    ///creates transition and if state does not exist, it will be created (with empty markings) and as non initial state
    pub fn add_arc(&self, from_id: Option<String>, label: String, to_id: Option<String>){

        let from_state = self.add_state(from_id, false);
        
        let to_id = self.add_state(to_id.clone(), false).read().unwrap().id.clone();

        let mut from_state_guard = from_state.write().unwrap();
        from_state_guard.next_state.write().unwrap().insert(label, to_id);
    }

    ///returns state after replaying, the remaining binding sequence and the current Marking
    /// replays the binding sequence on the MDFA as long as possible
    /*
    Input: Current State of MDFA, Hash of abstracted Marking (that could potentially be stated in the given state), binding sequence to be replayed
    Output: State of the MDFA after replaying, rest of the binding sequence
     */
    pub fn replay_next_state(&self, current_state: State, markings: &Vec<Marking>, binding_sequence: &Vec<(String, HashMap<String, HashSet<String>>)>) 
        -> (State, Vec<(String, HashMap<String, HashSet<String>>)>, Vec<Marking>){

        let hash_marking = &hash_markings(markings, &*self.object_to_type);
        
        //base case:
        if binding_sequence.is_empty() {
            return (current_state, binding_sequence.clone(), markings.clone());
        }

        //if next binding is enabled in this state, execute it
        let guard = current_state.next_state.read().unwrap();
        if let Some(next_state_guard) = binding_sequence
            .first()
            .and_then(|(label, _)| guard.get(label))
            .and_then(|next_state_id| self.states.get(next_state_id))
        {
            let next_state = next_state_guard.read().unwrap();
            //check if marking is in next state
            if let Some(next_markings) = next_state.markings.get(hash_marking) {
                //if yes, then we can replay the next state
                self.replay_next_state(next_state.clone(), next_markings, &binding_sequence[1..].to_vec())
            }else{
                //if next marking is not in the next state, MDFA replay stops
                (current_state.clone(), binding_sequence.clone(), markings.clone())
            }
        }else{
            //next label is not included in this state => MDFA replay stops
            (current_state.clone(), binding_sequence.clone(), markings.clone())
        }
    }

    pub fn export_to_petgraph(&self) -> DiGraph<String, String> {
        //construct graph
        let mut graph = DiGraph::<String, String>::new();
        let mut nodes: HashMap<String, petgraph::graph::NodeIndex> = HashMap::new();

        let mut state_queue: VecDeque<State> = VecDeque::new();
        let initial_state:State = self.states.get(&self.initial_states.read().unwrap().iter().next().unwrap().clone()).unwrap().read().unwrap().clone();
        state_queue.push_back(initial_state);

        for state in self.states.iter(){
            let mut infos = "".to_string();
            infos.push_str(&format!("--------- {} ---------\n", state.read().unwrap().id.clone()));
            for (marking_hash, markings) in state.read().unwrap().markings.iter() {
                infos.push_str(&format!("{}: {:?}\n", marking_hash, markings));
            }
            infos.push_str("\n-------------------");
            for (hash, activities) in state.read().unwrap().enabled_activities.iter() {
                infos.push_str(&format!("\n{}: {:?}", hash, activities));
            }  
            nodes.insert(state.read().unwrap().id.clone(), graph.add_node(infos));

        }

        while let Some(state) = state_queue.pop_front() {


            for (label, next_state_id) in state.next_state.read().unwrap().iter() {

                if let Some(next_state) = self.states.get(next_state_id) {
                    state_queue.push_back(next_state.read().unwrap().clone());
                }


                let source = nodes.get(&state.id).expect("Source node not found");
                let target = nodes.get(next_state_id).expect("Target node not found");
                graph.add_edge(*source, *target, label.clone());


            }
        }
        graph
    }

    pub fn export_dot_to_file(&self, path:String){

        let graph = self.export_to_petgraph().clone();


        //visualize graph (export)
        //let dot = Dot::with_config(&graph, &[]);
        let dot = Dot::with_attr_getters(
            &graph,
            &[],

            // edge attributes
            &|_graph, edge_ref| {
                //get edge attributes
                let mut x = String::new();
                let edge_weight = edge_ref.weight();
                if let label = edge_weight {
                    if label.starts_with("offer") {
                        x = format!("color=\"#{}\", style=filled", "0000ff");
                    } else if label.starts_with("application") {
                        x = format!("color=\"#{}\", style=filled", "ff0000");
                    }
                }
                x
            },

            // Node attributes
            &|_graph, node_ref| {

                let node_label = node_ref.1.clone();
                let mut x:String = "".to_string();

                if node_label.starts_with("offer") {
                    x = format!("color = \"#{}\", style =filled", "6666ff");
                }else if node_label.starts_with("application") {
                    x = format!("color=\"#{}\", style=filled", "ff6666");
                }else if node_label.contains(":") {
                    x = format!("shape = box; style = filled; color = gray");
                }else {
                    x = format!(" shape = box; style = filled; color = \"#575454\"");
                }

                x
            },
        );

        //save eog to vis.dot
        let mut file = File::create(path.clone()).expect("error");
        write!(file, "{}", dot).expect("error");

        println!("graph saved in {:?}; run with: dot -Tpng {} -o [file name].png", file, path);

    }
}

pub fn get_all_objects_of_marking(marking: Marking, object_to_type: &HashMap<String, String>) -> HashMap<String, HashSet<String>> {
    let mut objects: HashMap<String, HashSet<String>> = HashMap::new();
    for (_, _objects) in marking.iter() {
        //all object in one place have same type
        let object_type = object_to_type.get(_objects.iter().next().unwrap()).expect("Object type not found");
        objects.entry(object_type.clone())
            .or_insert_with(HashSet::new)
            .extend(_objects.iter().cloned());
    }
    objects
}

pub fn abstract_marking_with_map(marking: &Marking, object_rename: &HashMap<String, String>) -> Marking{
    let mut result: Marking = marking.clone();
    //edit marking
    for (_, objects) in result.iter_mut() {
        let mut new_objs = HashSet::with_capacity(objects.len());
        for obj in objects.drain() {
            let rename = object_rename
                .get(&obj)
                .unwrap_or_else(|| panic!("Object {} not found in rename map", obj))
                .clone();
            new_objs.insert(rename);
        }
        *objects = new_objs;
    }
    result
}

pub fn abstract_binding_sequnece_with_map(binding_sequence: &Vec<(String, HashMap<String, HashSet<String>>)>, object_rename: &HashMap<String, String>) -> Vec<(String, HashMap<String, HashSet<String>>)>{
    let mut result: Vec<(String, HashMap<String, HashSet<String>>)> = Vec::with_capacity(binding_sequence.len());
    for (label, binding) in binding_sequence.iter() {
        let mut new_binding: HashMap<String, HashSet<String>> = HashMap::with_capacity(binding.len());
        for (ot, objects) in binding.iter() {
            let mut new_objs = HashSet::with_capacity(objects.len());
            for obj in objects.iter() {
                let rename = object_rename
                    .get(obj)
                    .unwrap_or_else(|| panic!("Object {} not found in rename map", obj))
                    .clone();
                new_objs.insert(rename);
            }
            new_binding.insert(ot.clone(), new_objs);
        }
        result.push((label.clone(), new_binding));
    }
    result
}

///get all possible rename maps for a marking
pub fn get_all_rename_maps(marking: &Marking, object_to_type: &HashMap<String, String>) -> Vec<HashMap<String, String>> {

    let mut rename_maps:Vec<HashMap<String, String>> = Vec::new();

    let mut objects_: HashMap<String, Vec<String>> = get_all_objects_of_marking(marking.clone(), object_to_type)
        .into_iter()
        .map(|(k, v)| (k, v.into_iter().collect()))
        .collect();
    
    let DEBUG_OBJECTS = objects_.clone();

    let mut permutations: HashMap<String, Vec<Vec<&String>>> = HashMap::with_capacity(objects_.len());
    //sort for consistency
    for (ot, objects) in objects_.iter_mut() {
        objects.sort();
        //get all permutations of objects for each type
        //println!("object length: {}", objects.len());
        if objects.len() > 20 {
            panic!("Warning: number of objects for type {} is {}, which may lead to a very large number of permutations ({}!). Consider using fewer objects for type: {}. \n\n objects: {:?}", ot, objects.len(), objects.len(), ot, DEBUG_OBJECTS);
        }
        let perms = objects.iter().permutations(objects.len()).collect_vec();
        permutations.entry(ot.clone()).or_insert_with(Vec::new).extend(perms);
    }


    //get all combinations of objects for each type
    for combination in permutations.values().into_iter().multi_cartesian_product().map(|combination| combination.to_vec()) {

        let mut rename_map: HashMap<String, String> = HashMap::new();

        for objects in combination.iter() {
            //TODO: object type is already known, so we can use it to create a rename map; but lookup is O(1)?
            let object_type = object_to_type.get(*objects.first().unwrap()).unwrap();

            for (index, &obj) in objects.iter().enumerate() {
                let abstracted_object_id = format!("{}:{}", object_type, index);
                rename_map.insert(obj.clone(), abstracted_object_id.clone());
            }
        }
        rename_maps.push(rename_map);
    }
    rename_maps
}


///abstracts a marking to a more generic representation while preserving object identity
/// not very efficient :(
pub fn abstract_marking_map(marking: &Marking, object_to_type: &HashMap<String, String>) -> (Marking, HashMap<String, String>){
    //convert object ids to more generic object ids

    //get all objects in the marking
    let mut objects: HashMap<String, HashSet<String>> = get_all_objects_of_marking(marking.clone(), object_to_type);

    //get rename map
    let mut object_rename: HashMap<String, String> = HashMap::new();
    for (object_type, objects) in objects.iter_mut() {

        for (index, _obj) in objects.iter().enumerate() {
            let abstracted_object_id = format!("{}:{}", object_type, index);
            object_rename.insert(_obj.clone(), abstracted_object_id.clone());
        }
    }
    //edit marking
    (abstract_marking_with_map(marking, &object_rename), object_rename)
}

pub fn abstract_marking(marking: &Marking, object_to_type: &HashMap<String, String>) -> Marking {
    //convert object ids to more generic object ids
    let (abstracted_marking, _) = abstract_marking_map(marking, object_to_type);
    abstracted_marking
}



///check if both markings are equal if object ids are omitted
pub fn compare_marking_multiplicity(marking_a: &Marking, marking_b: &Marking, object_to_type: &HashMap<String, String>) -> bool {

    //check num of places
    if marking_a.len() != marking_b.len() {
        return false;
    }

    //check all objects
    for (place, objects_a) in marking_a.iter() {
        if let Some(objects_b) = marking_b.get(place) {
            //check object type:
            if get_object_type(&objects_a.iter().next().unwrap().clone(), object_to_type) != get_object_type(&objects_b.iter().next().unwrap().clone(), object_to_type) {
                println!("ot_a: {:?}, ot_b: {:?}", object_to_type.get(objects_a.iter().next().unwrap()), object_to_type.get(objects_b.iter().next().unwrap()));
                return false;
            }

            //chekc objects
            if objects_a.len() != objects_b.len() {
                return false;
            }

        } else { return false; }
    }
    true
}

pub fn get_object_type(object: &String, object_to_type: &HashMap<String, String>) -> String{
    //just take everything before the first :
    if let Some(ot) = object_to_type.get(object) {
        ot.clone()
    } else {
        //if object is not in the map, it must be abstracted
        get_object_type_of_abstracted(object)
    }
}
pub fn get_object_type_of_abstracted(object: &String) -> String{
    if let Some(index) = object.find(':') {
        object[..index].to_string()
    } else {
        object.clone() // if no : found, return the object as is
    }
}

