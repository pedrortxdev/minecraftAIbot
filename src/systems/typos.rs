use rand::Rng;
use crate::cognitive::personality::Mood;

// ============================================================
// TYPOS MIDDLEWARE — "Fat Finger" Filter
// Makes Gemini output look like a real player typed it
// ============================================================

/// Process Gemini output to add realistic typos
pub fn apply_typos(text: &str, mood: &Mood) -> String {
    let mut rng = rand::thread_rng();
    let mut result = text.to_string();

    // === 1. ALWAYS: Remove trailing punctuation (ponto final é coisa de psicopata) ===
    result = result.trim_end_matches('.').trim_end().to_string();
    result = result.trim_end_matches('!').trim_end().to_string(); // Only if not shouting

    // === 2. ALWAYS: Lowercase everything (unless SCREAMING mood) ===
    if *mood != Mood::Annoyed && *mood != Mood::Hyped {
        result = result.to_lowercase();
    }

    // === 3. Remove accents sometimes (lazy typing) ===
    if rng.r#gen::<f32>() < 0.4 {
        result = remove_some_accents(&result, &mut rng);
    }

    // === 4. Swap adjacent letters (fat finger) ===
    let typo_chance = match mood {
        Mood::Scared | Mood::Hyped => 0.15,   // Typing fast = more typos
        Mood::Focused => 0.03,                  // Careful typing
        Mood::Grumpy => 0.10,                   // Annoyed, sloppy
        _ => 0.07,                              // Normal
    };

    result = maybe_swap_letters(&result, typo_chance, &mut rng);

    // === 5. Double letters (sticky keys) ===
    if rng.r#gen::<f32>() < 0.08 {
        result = double_random_letter(&result, &mut rng);
    }

    // === 6. Drop random letters ===
    if rng.r#gen::<f32>() < 0.06 {
        result = drop_random_letter(&result, &mut rng);
    }

    // === 7. Abbreviations (player chat shortcuts) ===
    result = apply_abbreviations(&result, &mut rng);

    // === 8. Mood-specific additions ===
    match mood {
        Mood::Hyped => {
            if rng.r#gen::<f32>() < 0.3 {
                result.push_str(" kkkk");
            }
        }
        Mood::Annoyed => {
            if rng.r#gen::<f32>() < 0.2 {
                result.push_str(" pqp");
            }
        }
        Mood::Scared => {
            if rng.r#gen::<f32>() < 0.25 {
                result = result.to_uppercase(); // PANIC CAPS
            }
        }
        _ => {}
    }

    // === 9. Random "kkk" laugh or filler ===
    if rng.r#gen::<f32>() < 0.05 {
        let fillers = ["kkk", "nn", "ss", "hm"];
        let filler = fillers[rng.r#gen::<usize>() % fillers.len()];
        result.push(' ');
        result.push_str(filler);
    }

    result.trim().to_string()
}

/// Swap two adjacent characters at a random position
fn maybe_swap_letters(text: &str, chance: f32, rng: &mut impl Rng) -> String {
    if text.len() < 4 {
        return text.to_string();
    }

    let chars: Vec<char> = text.chars().collect();
    let mut result: Vec<char> = chars.clone();
    let mut swapped = false;

    for i in 1..result.len() - 1 {
        if rng.r#gen::<f32>() < chance && result[i].is_alphabetic() && result[i + 1].is_alphabetic() && !swapped {
            result.swap(i, i + 1);
            swapped = true; // Only one swap per message
        }
    }

    result.iter().collect()
}

/// Double a random letter (sticky key)
fn double_random_letter(text: &str, rng: &mut impl Rng) -> String {
    let chars: Vec<char> = text.chars().collect();
    if chars.len() < 3 {
        return text.to_string();
    }

    let idx = rng.r#gen::<usize>() % (chars.len() - 1) + 1;
    if chars[idx].is_alphabetic() {
        let mut result = String::new();
        for (i, c) in chars.iter().enumerate() {
            result.push(*c);
            if i == idx {
                result.push(*c); // Double it
            }
        }
        result
    } else {
        text.to_string()
    }
}

/// Drop a random letter
fn drop_random_letter(text: &str, rng: &mut impl Rng) -> String {
    let chars: Vec<char> = text.chars().collect();
    if chars.len() < 5 {
        return text.to_string();
    }

    let idx = rng.r#gen::<usize>() % (chars.len() - 2) + 1; // Don't drop first or last
    if chars[idx].is_alphabetic() && chars[idx] != ' ' {
        let mut result = String::new();
        for (i, c) in chars.iter().enumerate() {
            if i != idx {
                result.push(*c);
            }
        }
        result
    } else {
        text.to_string()
    }
}

/// Remove some accents lazily
fn remove_some_accents(text: &str, rng: &mut impl Rng) -> String {
    text.chars()
        .map(|c| {
            if rng.r#gen::<f32>() < 0.5 {
                match c {
                    'á' | 'à' | 'ã' | 'â' => 'a',
                    'é' | 'è' | 'ê' => 'e',
                    'í' | 'ì' => 'i',
                    'ó' | 'ò' | 'õ' | 'ô' => 'o',
                    'ú' | 'ù' => 'u',
                    'ç' => 'c',
                    'Á' | 'À' | 'Ã' | 'Â' => 'A',
                    'É' | 'È' | 'Ê' => 'E',
                    'Í' | 'Ì' => 'I',
                    'Ó' | 'Ò' | 'Õ' | 'Ô' => 'O',
                    'Ú' | 'Ù' => 'U',
                    'Ç' => 'C',
                    other => other,
                }
            } else {
                c
            }
        })
        .collect()
}

/// Apply common chat abbreviations
fn apply_abbreviations(text: &str, rng: &mut impl Rng) -> String {
    let mut result = text.to_string();

    let replacements = [
        ("porque", "pq"),
        ("também", "tb"),
        ("você", "vc"),
        ("voce", "vc"),
        ("não", "n"),
        ("nao", "n"),
        ("para", "pra"),
        ("está", "ta"),
        ("esta", "ta"),
        ("estou", "to"),
        ("muito", "mt"),
        ("quando", "qnd"),
        ("quanto", "qnt"),
        ("aqui", "aki"),
        ("beleza", "blz"),
        ("tranquilo", "tranks"),
        ("obrigado", "vlw"),
        ("obrigada", "vlw"),
        ("verdade", "vdd"),
        ("comigo", "cmg"),
        ("contigo", "ctg"),
        ("demais", "dms"),
    ];

    for (from, to) in &replacements {
        if rng.r#gen::<f32>() < 0.6 { // 60% chance to abbreviate
            result = result.replace(from, to);
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_typos_basic() {
        let input = "Eu preciso de redstone.";
        let output = apply_typos(input, &Mood::Chill);
        // Should be lowercase and no period
        assert!(!output.ends_with('.'));
        println!("Input:  {}", input);
        println!("Output: {}", output);
    }

    #[test]
    fn test_typos_scared() {
        let input = "Tem muito mob aqui";
        for _ in 0..5 {
            let output = apply_typos(input, &Mood::Scared);
            println!("Scared: {}", output);
        }
    }

    #[test]
    fn test_abbreviations() {
        let input = "porque você não está aqui comigo";
        let output = apply_typos(input, &Mood::Chill);
        println!("Abbrev: {}", output);
        // Should have some abbreviations
    }
}
