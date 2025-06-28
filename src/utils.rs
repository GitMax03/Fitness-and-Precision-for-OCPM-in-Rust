use std::collections::{HashMap, HashSet};
use std::time::Instant;
use chrono::Duration;
use counter::Counter;
use crate::NUMBER_OF_ITERATIONS;

pub fn evaluate_runtime(starting_times:&Vec<Instant>, steps_as_strings:&Vec<String>){
    
    let endtime = std::time::Instant::now();
    
    println!("\n\n###############################################################");
    println!("evaluating runtime ...");
    println!("###############################################################\n\n");
    
    if starting_times.len() != steps_as_strings.len() + 1 {
        println!("Steps length mismatch.");
        
        println!("Starting times:{:?}", starting_times);
        println!("Steps:{:?}", steps_as_strings);
        return;
    }
    
    let mut task_time_map:Vec<(String,f64)> = Vec::new();

    for index in (1..starting_times.len()) {
        
        //calc elapes time for this step
        let elapsed_time = (starting_times[index] - starting_times[index - 1]) / NUMBER_OF_ITERATIONS.into();
        
        task_time_map.push((steps_as_strings[index - 1].to_string(), elapsed_time.as_secs_f64()));
        
        println!("{:<50} took {} seconds", steps_as_strings[index - 1], elapsed_time.as_secs_f64());
    }
    
    println!("----------------------------------------------------------------------------------------");


    
    task_time_map.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    for (name, time) in &task_time_map {
        println!("{:<50} : {} seconds", name, time);
    }

    println!("----------------------------------------------------------------------------------------");
    let total_time = endtime - starting_times.first().unwrap().clone();
    
    println!("Total time: {} Seconds With {} iterations", total_time.as_secs_f64(), NUMBER_OF_ITERATIONS);
    println!("Average time: {} Seconds", (total_time.as_secs_f64() / NUMBER_OF_ITERATIONS as f64));
    println!("----------------------------------------------------------------------------------------");
    
    let mut percentages: Vec<(String,f32)> = Vec::new();
    let sum = task_time_map.iter().map(|(a,b)|b).sum::<f64>();
    for (task, time) in task_time_map{
        percentages.push((task, (time / sum * 100.09) as f32));
    }
    percentages.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    for (name, percentage) in &percentages {
        println!("{:<50} : {} %", name, percentage);
    }
    
    println!("\n\n #####################################################################################\n\n\n")
    
}


//TODO provably already exists
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
