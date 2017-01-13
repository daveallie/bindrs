use regex::RegexSet;

pub fn process_ignores(vec: &mut Vec<String>) -> RegexSet {
    if vec.len() == 0 {
        vec.push("^\\.git(?:/[^/]+)*$".to_owned());
    }
    vec.push("^\\.bindrs.*$".to_owned());

    vec_to_regex_set(&vec)
}

fn vec_to_regex_set(ignores: &Vec<String>) -> RegexSet {
    let mut regexes: Vec<String> = vec![];

    for i in ignores.iter() {
        let mut ignore = i.clone();
        if !(ignore.starts_with("^") && ignore.ends_with("$")) {
            ignore = format!("^{}(?:/[^/]+)*$", ignore);
        }
        regexes.push(ignore)
    }

    RegexSet::new(&regexes[..]).unwrap()
}
