use std::collections::{HashMap, HashSet};
use counter::Counter;

pub fn has_intersection(a: &[String], b: &[String]) -> bool {
    // no _is_disjoint func for vec?
    let set_a: HashSet<_> = a.iter().collect();  
    b.iter().any(|s| set_a.contains(s))         
}



//check if a is superset of b (i.e. a contains all elements of b)
//TODO exisits already
pub fn is_superset(superset: &Vec<String>, subset: &Vec<String>) -> bool {
    let superset_set: HashSet<_> = superset.iter().collect();
    subset.iter().all(|item| superset_set.contains(item))
}

//TODO: mybe change vec<String> to HashSet<String>? for better efficiency
pub fn pop_object_from_binding(binding: &mut (String, Vec<String>), object_type: &String, object_to_type: &HashMap<String, String>) -> Option<String> {

    // get position of first object that matches the object_type
    let pos = binding.1.iter().position(|x| object_to_type.get(x).unwrap() == object_type);

    // pop the object at that position
    pos.map(|p| binding.1.remove(p))
    
}



//-----pretty print functions for debug-----------
pub fn print_contexts(contexts: &HashMap<String, HashMap<String, Counter<Vec<String>>>>) {
    
    //sort contexts by event_id
    let sorted_contexts = sort_hashmap_by_numeric_keys::<String, HashMap<String, Counter<Vec<String>>>>(&contexts);
    
    
    
    println!("Contexts:");
    for (event_type, context_map) in sorted_contexts {
        println!("Event Type: {}", event_type);
        for (sorted_contexts, bindings) in context_map {
            println!("  Context: {}", sorted_contexts);
            for (binding, count) in bindings.iter() {
                println!("    Activities: {:?}, Count: {}", binding, count);
            }
        }
    }
    println!("____________________________________________");
}


pub fn sort_hashmap_by_numeric_keys<K, V>(
    data: &HashMap<String, V>,
) -> Vec<(&String, &V)> {
    let mut entries: Vec<_> = data.iter().collect();

    entries.sort_by_key(|(key, _)| {
        key.chars()
            .filter_map(|c| c.to_digit(10))  // Extrahiere einzelne Ziffern
            .fold(0, |acc, digit| acc * 10 + digit)  // Kombiniere zu einer Zahl
    });

    entries
}
