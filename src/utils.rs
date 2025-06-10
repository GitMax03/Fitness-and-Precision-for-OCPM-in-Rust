use std::collections::HashSet;

pub fn has_intersection(a: &[String], b: &[String]) -> bool {
    // no _is_disjoint func for vec?
    let set_a: HashSet<_> = a.iter().collect();  
    b.iter().any(|s| set_a.contains(s))         
}

//check if a is superset of b (i.e. a contains all elements of b)
pub fn is_superset(superset: Vec<String>, subset: &Vec<String>) -> bool {
    let superset_set: HashSet<_> = superset.iter().collect();
    subset.iter().all(|item| superset_set.contains(item))
}

