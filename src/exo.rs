//! Exercices guidés avec vérification réelle (compile avec rustc + tests).
//!
//! Modèle : la solution de l'étudiant vit dans `exo/<id>/<id>.rs` (fonctions
//! `pub`). Le harnais `_test.rs` l'importe via `#[path]` et lance des tests
//! dans un `fn main()` — compilé avec `rustc --edition 2021`.

/// Un exercice : la tâche, le squelette à compléter, le harnais de test.
pub struct Exo {
    pub id: &'static str,
    pub title: &'static str,
    pub difficulty: u8,
    pub task: &'static str,
    /// La signature imposée.
    pub signature: &'static str,
    /// Le code de départ (à écrire dans le fichier de l'exo).
    pub starter: &'static str,
    /// Le harnais de test (main + assertions) — compilé avec la solution.
    /// Les fonctions de la solution sont accessibles directement (use solution::*).
    pub harness: &'static str,
    /// Indices progressifs (du plus léger au plus direct).
    pub hints: &'static [&'static str],
    /// Fiche liée pour la théorie.
    pub fiche: &'static str,
}

pub const EXOS: &[Exo] = &[
    Exo {
        id: "fizzbuzz",
        title: "fizzbuzz, le classique",
        difficulty: 1,
        task: "Écris `pub fn fizzbuzz(n: u32) -> Vec<String>` qui renvoie les valeurs de 1 à n : \
               \"fizz\" si multiple de 3, \"buzz\" si multiple de 5, \"fizzbuzz\" si les deux, \
               sinon le nombre en texte.",
        signature: "pub fn fizzbuzz(n: u32) -> Vec<String>",
        starter: "pub fn fizzbuzz(n: u32) -> Vec<String> {\n    let _ = n;\n    todo!(\"à toi\")\n}\n",
        harness: r#"fn check(n: u32, attendu: &[&str]) -> bool {
    let got = fizzbuzz(n);
    let attendu: Vec<String> = attendu.iter().map(|s| s.to_string()).collect();
    if got == attendu {
        println!("✓ fizzbuzz({})", n);
        true
    } else {
        println!("✗ fizzbuzz({}) = {:?}, attendu {:?}", n, got, attendu);
        false
    }
}

fn main() {
    let mut ok = true;
    ok &= check(5, &["1", "2", "fizz", "4", "buzz"]);
    ok &= check(15, &["1", "2", "fizz", "4", "buzz", "fizz", "7", "8", "fizz", "buzz", "11", "fizz", "13", "14", "fizzbuzz"]);
    ok &= check(0, &[]);
    std::process::exit(if ok { 0 } else { 1 });
}
"#,
        hints: &[
            "Boucle de 1 à n INCLUS (range inclusif), pousse dans un Vec<String>.",
            "Teste la divisibilité par 15 AVANT 3 et 5 séparément.",
            "n.to_string() convertit le nombre ; (1..=n) donne le range inclusif.",
        ],
        fiche: "boucles",
    },
    Exo {
        id: "factorielle",
        title: "factorielle récursive",
        difficulty: 1,
        task: "Écris `pub fn factorielle(n: u32) -> u64` qui renvoie n! de façon RÉCURSIVE. \
               factorielle(0) vaut 1.",
        signature: "pub fn factorielle(n: u32) -> u64",
        starter: "pub fn factorielle(n: u32) -> u64 {\n    let _ = n;\n    todo!(\"à toi\")\n}\n",
        harness: r#"fn check(n: u32, attendu: u64) -> bool {
    let got = factorielle(n);
    if got == attendu {
        println!("✓ factorielle({}) = {}", n, got);
        true
    } else {
        println!("✗ factorielle({}) = {}, attendu {}", n, got, attendu);
        false
    }
}

fn main() {
    let mut ok = true;
    ok &= check(0, 1);
    ok &= check(1, 1);
    ok &= check(5, 120);
    ok &= check(10, 3628800);
    std::process::exit(if ok { 0 } else { 1 });
}
"#,
        hints: &[
            "factorielle(n) = n × factorielle(n - 1). Quel est le cas d'arrêt ?",
            "if n == 0 { 1 } else { n as u64 * factorielle(n - 1) }",
        ],
        fiche: "fonctions",
    },
    Exo {
        id: "fibonacci",
        title: "fibonacci itératif",
        difficulty: 2,
        task: "Écris `pub fn fibonacci(n: u32) -> u64` qui renvoie le nième nombre de Fibonacci \
               (fib(0) = 0, fib(1) = 1), de façon ITÉRATIVE (une boucle, deux variables).",
        signature: "pub fn fibonacci(n: u32) -> u64",
        starter: "pub fn fibonacci(n: u32) -> u64 {\n    let _ = n;\n    todo!(\"à toi\")\n}\n",
        harness: r#"fn check(n: u32, attendu: u64) -> bool {
    let got = fibonacci(n);
    if got == attendu {
        println!("✓ fibonacci({}) = {}", n, got);
        true
    } else {
        println!("✗ fibonacci({}) = {}, attendu {}", n, got, attendu);
        false
    }
}

fn main() {
    let mut ok = true;
    ok &= check(0, 0);
    ok &= check(1, 1);
    ok &= check(2, 1);
    ok &= check(7, 13);
    ok &= check(20, 6765);
    std::process::exit(if ok { 0 } else { 1 });
}
"#,
        hints: &[
            "Deux variables a et b (0 et 1) ; à chaque tour : le nouveau a c'est b, le nouveau b c'est a + b.",
            "Rust permet l'échange en une ligne : (a, b) = (b, a + b); — une affectation de tuple.",
        ],
        fiche: "boucles",
    },
    Exo {
        id: "palindrome",
        title: "détecter un palindrome",
        difficulty: 2,
        task: "Écris `pub fn est_palindrome(s: &str) -> bool` qui renvoie true si `s` se lit \
               pareil dans les deux sens (caractère par caractère). La chaîne vide est un palindrome.",
        signature: "pub fn est_palindrome(s: &str) -> bool",
        starter: "pub fn est_palindrome(s: &str) -> bool {\n    let _ = s;\n    todo!(\"à toi\")\n}\n",
        harness: r#"fn check(s: &str, attendu: bool) -> bool {
    let got = est_palindrome(s);
    if got == attendu {
        println!("✓ est_palindrome({:?}) = {}", s, got);
        true
    } else {
        println!("✗ est_palindrome({:?}) = {}, attendu {}", s, got, attendu);
        false
    }
}

fn main() {
    let mut ok = true;
    ok &= check("", true);
    ok &= check("a", true);
    ok &= check("kayak", true);
    ok &= check("ressasser", true);
    ok &= check("bonjour", false);
    ok &= check("ab", false);
    std::process::exit(if ok { 0 } else { 1 });
}
"#,
        hints: &[
            "On ne peut PAS indexer une &str (UTF-8) : passe par les caractères.",
            "s.chars() te donne un itérateur de char… qui est doublement terminé : .next() et .next_back().",
            "Ou plus court : compare s.chars() avec s.chars().rev() — un itérateur se compare avec eq().",
        ],
        fiche: "slices",
    },
    Exo {
        id: "renverser",
        title: "renverser une chaîne",
        difficulty: 1,
        task: "Écris `pub fn renverser(s: &str) -> String` qui renvoie une NOUVELLE String \
               avec les caractères de `s` dans l'ordre inverse (Unicode-safe : caractères, pas octets).",
        signature: "pub fn renverser(s: &str) -> String",
        starter: "pub fn renverser(s: &str) -> String {\n    let _ = s;\n    todo!(\"à toi\")\n}\n",
        harness: r#"fn check(s: &str, attendu: &str) -> bool {
    let got = renverser(s);
    if got == attendu {
        println!("✓ renverser({:?}) = {:?}", s, got);
        true
    } else {
        println!("✗ renverser({:?}) = {:?}, attendu {:?}", s, got, attendu);
        false
    }
}

fn main() {
    let mut ok = true;
    ok &= check("", "");
    ok &= check("a", "a");
    ok &= check("bonjour", "ruojnob");
    ok &= check("rust", "tsur");
    ok &= check("été", "été");  // les caractères, pas les octets
    std::process::exit(if ok { 0 } else { 1 });
}
"#,
        hints: &[
            "s.chars() itère sur les caractères ; .rev() inverse l'itérateur.",
            "Un itérateur de char se collecte en String : .collect().",
            "s.chars().rev().collect() — trois appels, une ligne.",
        ],
        fiche: "string-manipulation",
    },
    Exo {
        id: "moyenne",
        title: "moyenne d'un tableau (Option)",
        difficulty: 2,
        task: "Écris `pub fn moyenne(v: &[i32]) -> Option<f64>` qui renvoie la moyenne des \
               éléments, ou None si la slice est vide (pas de division par zéro chez toi).",
        signature: "pub fn moyenne(v: &[i32]) -> Option<f64>",
        starter: "pub fn moyenne(v: &[i32]) -> Option<f64> {\n    let _ = v;\n    todo!(\"à toi\")\n}\n",
        harness: r#"fn check(v: &[i32], attendu: Option<f64>) -> bool {
    let got = moyenne(v);
    let ok = match (got, attendu) {
        (None, None) => true,
        (Some(a), Some(b)) => (a - b).abs() < 1e-9,
        _ => false,
    };
    if ok {
        println!("✓ moyenne({:?}) = {:?}", v, got);
        true
    } else {
        println!("✗ moyenne({:?}) = {:?}, attendu {:?}", v, got, attendu);
        false
    }
}

fn main() {
    let mut ok = true;
    ok &= check(&[], None);
    ok &= check(&[10], Some(10.0));
    ok &= check(&[1, 2, 3, 4], Some(2.5));
    ok &= check(&[-2, 2], Some(0.0));
    std::process::exit(if ok { 0 } else { 1 });
}
"#,
        hints: &[
            "Vide → None. Sinon Some(somme / taille).",
            "v.iter().sum::<i32>() pour la somme ; convertis en f64 AVANT de diviser (as f64).",
            "Some(v.iter().sum::<i32>() as f64 / v.len() as f64)",
        ],
        fiche: "option",
    },
    Exo {
        id: "maximum",
        title: "le max d'une slice (Option)",
        difficulty: 2,
        task: "Écris `pub fn maximum(v: &[i32]) -> Option<i32>` qui renvoie le plus grand élément, \
               ou None si la slice est vide. Sans utiliser .max() — fais-le à la main.",
        signature: "pub fn maximum(v: &[i32]) -> Option<i32>",
        starter: "pub fn maximum(v: &[i32]) -> Option<i32> {\n    let _ = v;\n    todo!(\"à toi\")\n}\n",
        harness: r#"fn check(v: &[i32], attendu: Option<i32>) -> bool {
    let got = maximum(v);
    if got == attendu {
        println!("✓ maximum({:?}) = {:?}", v, got);
        true
    } else {
        println!("✗ maximum({:?}) = {:?}, attendu {:?}", v, got, attendu);
        false
    }
}

fn main() {
    let mut ok = true;
    ok &= check(&[], None);
    ok &= check(&[42], Some(42));
    ok &= check(&[1, 9, 3, 7], Some(9));
    ok &= check(&[-5, -1, -9], Some(-1));
    std::process::exit(if ok { 0 } else { 1 });
}
"#,
        hints: &[
            "Vide → None. Sinon pars du premier élément et compare chaque suivant.",
            "v.first() te donne Option<&i32> directement… ou v[0] après le test de vide.",
            "for &x in &v[1..] — on peut itérer sur une slice par référence, le & déstructure.",
        ],
        fiche: "option",
    },
    Exo {
        id: "division_sure",
        title: "division qui ne panique jamais (Result)",
        difficulty: 2,
        task: "Écris `pub fn diviser(a: f64, b: f64) -> Result<f64, String>` : Ok(a / b) si b != 0.0, \
               sinon Err avec un message clair. Jamais de panique, jamais d'infini silencieux.",
        signature: "pub fn diviser(a: f64, b: f64) -> Result<f64, String>",
        starter: "pub fn diviser(a: f64, b: f64) -> Result<f64, String> {\n    let _ = (a, b);\n    todo!(\"à toi\")\n}\n",
        harness: r#"fn main() {
    let mut ok = true;
    match diviser(10.0, 2.0) {
        Ok(v) if (v - 5.0).abs() < 1e-9 => println!("✓ diviser(10, 2) = 5"),
        autre => { println!("✗ diviser(10, 2) = {:?}", autre); ok = false; }
    }
    match diviser(1.0, 0.0) {
        Err(e) if !e.is_empty() => println!("✓ diviser par 0 → Err({:?})", e),
        autre => { println!("✗ diviser par 0 devrait être Err, pas {:?}", autre); ok = false; }
    }
    match diviser(-9.0, 3.0) {
        Ok(v) if (v + 3.0).abs() < 1e-9 => println!("✓ diviser(-9, 3) = -3"),
        autre => { println!("✗ diviser(-9, 3) = {:?}", autre); ok = false; }
    }
    std::process::exit(if ok { 0 } else { 1 });
}
"#,
        hints: &[
            "if b == 0.0 { Err(...) } else { Ok(a / b) }",
            "Err(format!(\"division par zéro ({} / 0)\", a)) pour un message parlant.",
        ],
        fiche: "result",
    },
    Exo {
        id: "parse_addition",
        title: "additionner deux chaînes (l'opérateur ?)",
        difficulty: 3,
        task: "Écris `pub fn additionner(a: &str, b: &str) -> Result<i32, std::num::ParseIntError>` \
               qui parse les deux chaînes en i32 et renvoie leur somme. Utilise l'opérateur ? \
               pour propager les erreurs de parse — pas un seul match, pas un seul unwrap.",
        signature: "pub fn additionner(a: &str, b: &str) -> Result<i32, std::num::ParseIntError>",
        starter: "pub fn additionner(a: &str, b: &str) -> Result<i32, std::num::ParseIntError> {\n    let _ = (a, b);\n    todo!(\"à toi\")\n}\n",
        harness: r#"fn main() {
    let mut ok = true;
    match additionner("2", "40") {
        Ok(42) => println!("✓ \"2\" + \"40\" = 42"),
        autre => { println!("✗ additionner(\"2\", \"40\") = {:?}", autre); ok = false; }
    }
    if additionner("abc", "1").is_err() {
        println!("✓ \"abc\" + \"1\" → Err (propagée)");
    } else {
        println!("✗ \"abc\" devrait donner Err"); ok = false;
    }
    if additionner("1", "xyz").is_err() {
        println!("✓ \"1\" + \"xyz\" → Err (propagée)");
    } else {
        println!("✗ \"xyz\" devrait donner Err"); ok = false;
    }
    match additionner("-5", "5") {
        Ok(0) => println!("✓ \"-5\" + \"5\" = 0"),
        autre => { println!("✗ additionner(\"-5\", \"5\") = {:?}", autre); ok = false; }
    }
    std::process::exit(if ok { 0 } else { 1 });
}
"#,
        hints: &[
            "str.parse::<i32>() renvoie Result<i32, ParseIntError> — exactement le bon type d'erreur.",
            "let x: i32 = a.parse()?; — le ? sort de la fonction avec l'Err si le parse échoue.",
            "let x = a.parse::<i32>()?; let y = b.parse::<i32>()?; Ok(x + y)",
        ],
        fiche: "operateur-question",
    },
    Exo {
        id: "compter_mots",
        title: "compter les mots (HashMap)",
        difficulty: 3,
        task: "Écris `pub fn compter_mots(texte: &str) -> std::collections::HashMap<String, usize>` \
               qui compte les occurrences de chaque mot (séparés par les espaces, en minuscules).",
        signature: "pub fn compter_mots(texte: &str) -> std::collections::HashMap<String, usize>",
        starter: "use std::collections::HashMap;\n\npub fn compter_mots(texte: &str) -> HashMap<String, usize> {\n    let _ = texte;\n    todo!(\"à toi\")\n}\n",
        harness: r#"fn main() {
    let mut ok = true;
    let m = compter_mots("le rust et le code");
    if m.get("le") == Some(&2) && m.get("rust") == Some(&1) && m.len() == 4 {
        println!("✓ comptage simple");
    } else {
        println!("✗ comptage simple : {:?}", m); ok = false;
    }
    let m2 = compter_mots("Rust rust RUST");
    if m2.get("rust") == Some(&3) && m2.len() == 1 {
        println!("✓ insensible à la casse");
    } else {
        println!("✗ casse : {:?}", m2); ok = false;
    }
    let m3 = compter_mots("");
    if m3.is_empty() {
        println!("✓ texte vide → map vide");
    } else {
        println!("✗ texte vide : {:?}", m3); ok = false;
    }
    std::process::exit(if ok { 0 } else { 1 });
}
"#,
        hints: &[
            "texte.split_whitespace() découpe aux espaces ; .to_lowercase() normalise la casse.",
            "L'entry API : *map.entry(mot).or_insert(0) += 1; — une seule ligne par mot.",
        ],
        fiche: "hashmap",
    },
    Exo {
        id: "filtrer_pairs",
        title: "pairs au carré (itérateurs)",
        difficulty: 3,
        task: "Écris `pub fn pairs_au_carre(v: &[i32]) -> Vec<i32>` qui garde les nombres PAIRS \
               et renvoie leurs carrés, en UNE chaîne d'itérateurs (filter + map + collect), \
               sans boucle for explicite.",
        signature: "pub fn pairs_au_carre(v: &[i32]) -> Vec<i32>",
        starter: "pub fn pairs_au_carre(v: &[i32]) -> Vec<i32> {\n    let _ = v;\n    todo!(\"à toi\")\n}\n",
        harness: r#"fn check(v: &[i32], attendu: &[i32]) -> bool {
    let got = pairs_au_carre(v);
    if got == attendu {
        println!("✓ pairs_au_carre({:?}) = {:?}", v, got);
        true
    } else {
        println!("✗ pairs_au_carre({:?}) = {:?}, attendu {:?}", v, got, attendu);
        false
    }
}

fn main() {
    let mut ok = true;
    ok &= check(&[], &[]);
    ok &= check(&[1, 2, 3, 4], &[4, 16]);
    ok &= check(&[5, 7], &[]);
    ok &= check(&[-2, 0, 3], &[4, 0]);
    std::process::exit(if ok { 0 } else { 1 });
}
"#,
        hints: &[
            "v.iter() donne des &i32 : filter(|x| *x % 2 == 0), map(|x| x * x), collect().",
            "Attention au déréférencement : dans la closure, x est &i32 → *x ou |&x|.",
        ],
        fiche: "iterateurs",
    },
    Exo {
        id: "point_distance",
        title: "struct Point et distance (impl)",
        difficulty: 2,
        task: "Définis `pub struct Point { pub x: f64, pub y: f64 }` et implémente \
               `pub fn new(x: f64, y: f64) -> Self` et `pub fn distance(&self, autre: &Point) -> f64` \
               (distance euclidienne).",
        signature: "pub struct Point { pub x: f64, pub y: f64 } + impl Point { new, distance(&self, autre: &Point) -> f64 }",
        starter: "pub struct Point {\n    pub x: f64,\n    pub y: f64,\n}\n\nimpl Point {\n    pub fn new(x: f64, y: f64) -> Self {\n        todo!(\"à toi\")\n    }\n\n    pub fn distance(&self, autre: &Point) -> f64 {\n        let _ = autre;\n        todo!(\"à toi\")\n    }\n}\n",
        harness: r#"fn main() {
    let mut ok = true;
    let a = Point::new(0.0, 0.0);
    let b = Point::new(3.0, 4.0);
    let d = a.distance(&b);
    if (d - 5.0).abs() < 1e-9 {
        println!("✓ distance (0,0)-(3,4) = 5");
    } else {
        println!("✗ distance = {}, attendu 5.0", d); ok = false;
    }
    let d2 = a.distance(&a);
    if d2 == 0.0 {
        println!("✓ distance à soi-même = 0");
    } else {
        println!("✗ distance à soi-même = {}", d2); ok = false;
    }
    let c = Point::new(-1.0, 0.0);
    if (a.distance(&c) - 1.0).abs() < 1e-9 {
        println!("✓ distance avec négatifs");
    } else {
        println!("✗ distance (0,0)-(-1,0) = {}", a.distance(&c)); ok = false;
    }
    std::process::exit(if ok { 0 } else { 1 });
}
"#,
        hints: &[
            "new : Self { x, y } (raccourci : le champ prend le nom du paramètre).",
            "distance : ((self.x - autre.x).powi(2) + (self.y - autre.y).powi(2)).sqrt()",
            "self est une référence (&self) : self.x lit le champ via l'auto-deref.",
        ],
        fiche: "methodes-impl",
    },
    Exo {
        id: "aire_formes",
        title: "enum Forme et aires (match)",
        difficulty: 2,
        task: "Définis `pub enum Forme { Cercle(f64), Rectangle(f64, f64), Carre(f64) }` et \
               `pub fn aire(&self) -> f64` qui calcule l'aire avec un match.",
        signature: "pub enum Forme { Cercle(f64), Rectangle(f64, f64), Carre(f64) } + impl Forme { aire(&self) -> f64 }",
        starter: "pub enum Forme {\n    Cercle(f64),\n    Rectangle(f64, f64),\n    Carre(f64),\n}\n\nimpl Forme {\n    pub fn aire(&self) -> f64 {\n        todo!(\"à toi\")\n    }\n}\n",
        harness: r#"fn proche(a: f64, b: f64) -> bool { (a - b).abs() < 1e-9 }

fn main() {
    let mut ok = true;
    let c = Forme::Cercle(1.0);
    if proche(c.aire(), std::f64::consts::PI) {
        println!("✓ cercle r=1 → π");
    } else {
        println!("✗ cercle r=1 → {}", c.aire()); ok = false;
    }
    let r = Forme::Rectangle(3.0, 4.0);
    if proche(r.aire(), 12.0) {
        println!("✓ rectangle 3×4 → 12");
    } else {
        println!("✗ rectangle → {}", r.aire()); ok = false;
    }
    let s = Forme::Carre(2.5);
    if proche(s.aire(), 6.25) {
        println!("✓ carré 2.5 → 6.25");
    } else {
        println!("✗ carré → {}", s.aire()); ok = false;
    }
    std::process::exit(if ok { 0 } else { 1 });
}
"#,
        hints: &[
            "match self { Forme::Cercle(r) => ..., Forme::Rectangle(l, h) => ..., Forme::Carre(c) => ... }",
            "π est dans std::f64::consts::PI.",
            "Les bindings r, l, h capturés depuis &self sont des &f64 — la multiplication auto-déférence, pas de panique.",
        ],
        fiche: "enums",
    },
    Exo {
        id: "compteur_closure",
        title: "un compteur en closure",
        difficulty: 3,
        task: "Écris `pub fn compteur() -> impl FnMut() -> u32` qui renvoie une closure : \
               chaque appel renvoie le nombre d'appels (1, puis 2, puis 3…).",
        signature: "pub fn compteur() -> impl FnMut() -> u32",
        starter: "pub fn compteur() -> impl FnMut() -> u32 {\n    let mut n = 0u32;\n    move || {\n        let _ = &mut n;\n        todo!(\"à toi\")\n    }\n}\n",
        harness: r#"fn main() {
    let mut ok = true;
    let mut c1 = compteur();
    let suite: Vec<u32> = (0..5).map(|_| c1()).collect();
    if suite == vec![1, 2, 3, 4, 5] {
        println!("✓ compteur 1→5");
    } else {
        println!("✗ suite : {:?}", suite); ok = false;
    }
    let mut c2 = compteur();
    let premier_c2 = c2();
    let sixieme_c1 = c1();
    if premier_c2 == 1 && sixieme_c1 == 6 {
        println!("✓ deux compteurs indépendants");
    } else {
        println!("✗ indépendance : c2()={}, c1()={}", premier_c2, sixieme_c1); ok = false;
    }
    std::process::exit(if ok { 0 } else { 1 });
}
"#,
        hints: &[
            "La closure capture un `let mut n = 0;`… et elle doit en prendre POSSESSION.",
            "move || { n += 1; n } — le mot-clé move transfère n dans la closure.",
            "impl FnMut() -> u32 dans le retour : « je renvoie quelque chose qui implémente FnMut ».",
        ],
        fiche: "closures",
    },
    Exo {
        id: "plus_long_mot",
        title: "le plus long de deux mots (lifetimes)",
        difficulty: 4,
        task: "Écris `pub fn plus_long<'a>(a: &'a str, b: &'a str) -> &'a str` qui renvoie la \
               plus longue des deux chaînes (en cas d'égalité, la première). La lifetime 'a est \
               OBLIGATOIRE : sans elle, rustc refuse.",
        signature: "pub fn plus_long<'a>(a: &'a str, b: &'a str) -> &'a str",
        starter: "pub fn plus_long<'a>(a: &'a str, b: &'a str) -> &'a str {\n    let _ = (a, b);\n    todo!(\"à toi\")\n}\n",
        harness: r#"fn main() {
    let mut ok = true;
    if plus_long("rust", "go") == "rust" {
        println!("✓ \"rust\" > \"go\"");
    } else {
        println!("✗ plus_long(rust, go)"); ok = false;
    }
    if plus_long("c", "python") == "python" {
        println!("✓ \"python\" > \"c\"");
    } else {
        println!("✗ plus_long(c, python)"); ok = false;
    }
    if plus_long("ab", "cd") == "ab" {
        println!("✓ égalité → le premier");
    } else {
        println!("✗ égalité"); ok = false;
    }
    let s = String::from("programmation");
    let r = plus_long(s.as_str(), "code");
    if r == "programmation" {
        println!("✓ fonctionne avec &String via as_str()");
    } else {
        println!("✗ avec String"); ok = false;
    }
    std::process::exit(if ok { 0 } else { 1 });
}
"#,
        hints: &[
            "Compare les longueurs : if a.len() >= b.len() { a } else { b }.",
            "La lifetime 'a dit : « le retour vit aussi longtemps que le plus court des deux emprunts ». La signature est donnée — reste le corps.",
        ],
        fiche: "lifetimes",
    },
    Exo {
        id: "swap_mut",
        title: "échanger via emprunts mutables",
        difficulty: 2,
        task: "Écris `pub fn echanger(a: &mut i32, b: &mut i32)` qui échange les valeurs \
               pointées. En Rust, pas de pointeurs bruts : des emprunts mutables exclusifs.",
        signature: "pub fn echanger(a: &mut i32, b: &mut i32)",
        starter: "pub fn echanger(a: &mut i32, b: &mut i32) {\n    let _ = (a, b);\n    todo!(\"à toi\")\n}\n",
        harness: r#"fn main() {
    let mut ok = true;
    let (mut x, mut y) = (1, 2);
    echanger(&mut x, &mut y);
    if x == 2 && y == 1 {
        println!("✓ 1,2 → 2,1");
    } else {
        println!("✗ x={}, y={}", x, y); ok = false;
    }
    let (mut m, mut n) = (-5, 42);
    echanger(&mut m, &mut n);
    if m == 42 && n == -5 {
        println!("✓ -5,42 → 42,-5");
    } else {
        println!("✗ m={}, n={}", m, n); ok = false;
    }
    let (mut p, mut q) = (7, 7);
    echanger(&mut p, &mut q);
    if p == 7 && q == 7 {
        println!("✓ valeurs égales");
    } else {
        ok = false;
    }
    std::process::exit(if ok { 0 } else { 1 });
}
"#,
        hints: &[
            "*a déréférence l'emprunt : tu lis et écris la valeur pointée.",
            "let tmp = *a; *a = *b; *b = tmp;",
            "Bonus idiome : std::mem::swap(a, b) fait exactement ça — mais fais-le à la main pour apprendre.",
        ],
        fiche: "emprunts-mutables",
    },
    Exo {
        id: "pile_generique",
        title: "une pile générique (Vec<T>)",
        difficulty: 3,
        task: "Définis `pub struct Pile<T> { elements: Vec<T> }` avec new(), push(&mut self, x: T), \
               pop(&mut self) -> Option<T>, len(&self) -> usize. Générique : elle doit marcher \
               pour i32 ET String.",
        signature: "pub struct Pile<T> { ... } + impl<T> Pile<T> { new, push, pop, len }",
        starter: "pub struct Pile<T> {\n    elements: Vec<T>,\n}\n\nimpl<T> Pile<T> {\n    pub fn new() -> Self {\n        todo!(\"à toi\")\n    }\n\n    pub fn push(&mut self, x: T) {\n        let _ = x;\n        todo!(\"à toi\")\n    }\n\n    pub fn pop(&mut self) -> Option<T> {\n        todo!(\"à toi\")\n    }\n\n    pub fn len(&self) -> usize {\n        todo!(\"à toi\")\n    }\n}\n",
        harness: r#"fn main() {
    let mut ok = true;
    let mut p: Pile<i32> = Pile::new();
    p.push(1); p.push(2); p.push(3);
    if p.len() == 3 && p.pop() == Some(3) && p.pop() == Some(2) && p.len() == 1 {
        println!("✓ pile d'entiers (LIFO)");
    } else {
        println!("✗ pile d'entiers"); ok = false;
    }
    let mut s: Pile<String> = Pile::new();
    s.push("rouille".to_string());
    if s.pop().as_deref() == Some("rouille") && s.pop().is_none() {
        println!("✓ pile de String + pop sur vide = None");
    } else {
        println!("✗ pile de String"); ok = false;
    }
    let vide: Pile<u8> = Pile::new();
    if vide.len() == 0 {
        println!("✓ pile vide");
    } else {
        ok = false;
    }
    std::process::exit(if ok { 0 } else { 1 });
}
"#,
        hints: &[
            "Vec a déjà tout : push, pop (qui renvoie Option<T>), len. Ta pile est une façade.",
            "new() → Self { elements: Vec::new() }. Les méthodes qui modifient prennent &mut self.",
        ],
        fiche: "generics",
    },
    Exo {
        id: "afficher_display",
        title: "impl Display pour un type",
        difficulty: 3,
        task: "Définis `pub struct Temperature { pub celsius: f64 }` et implémente le trait \
               std::fmt::Display pour qu'elle s'affiche « 21.5°C » (un chiffre après la virgule).",
        signature: "pub struct Temperature { pub celsius: f64 } + impl std::fmt::Display for Temperature",
        starter: "pub struct Temperature {\n    pub celsius: f64,\n}\n\nimpl std::fmt::Display for Temperature {\n    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {\n        let _ = (self, f);\n        todo!(\"à toi\")\n    }\n}\n",
        harness: r#"fn main() {
    let mut ok = true;
    let t = Temperature { celsius: 21.5 };
    if format!("{}", t) == "21.5°C" {
        println!("✓ format! utilise ton Display");
    } else {
        println!("✗ affichage : {:?}", format!("{}", t)); ok = false;
    }
    let z = Temperature { celsius: 0.0 };
    if format!("{}", z) == "0.0°C" {
        println!("✓ zéro");
    } else {
        println!("✗ zéro : {:?}", format!("{}", z)); ok = false;
    }
    let n = Temperature { celsius: -12.34 };
    if format!("{}", n) == "-12.3°C" {
        println!("✓ arrondi à un chiffre");
    } else {
        println!("✗ arrondi : {:?}", format!("{}", n)); ok = false;
    }
    std::process::exit(if ok { 0 } else { 1 });
}
"#,
        hints: &[
            "impl std::fmt::Display for Temperature { fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result { ... } }",
            "Dedans : write!(f, \"{:.1}°C\", self.celsius) — {:.1} = un chiffre après la virgule.",
        ],
        fiche: "traits",
    },
    Exo {
        id: "rot13",
        title: "chiffrement rot13",
        difficulty: 3,
        task: "Écris `pub fn rot13(s: &str) -> String` qui décale chaque lettre de 13 positions \
               dans l'alphabet (a↔n, b↔o…), en préservant la casse. Les non-lettres passent \
               telles quelles. (rot13 est sa propre réciproque : rot13(rot13(x)) == x.)",
        signature: "pub fn rot13(s: &str) -> String",
        starter: "pub fn rot13(s: &str) -> String {\n    let _ = s;\n    todo!(\"à toi\")\n}\n",
        harness: r#"fn main() {
    let mut ok = true;
    if rot13("hello") == "uryyb" {
        println!("✓ hello → uryyb");
    } else {
        println!("✗ hello → {}", rot13("hello")); ok = false;
    }
    if rot13("uryyb") == "hello" {
        println!("✓ réciproque");
    } else {
        ok = false;
    }
    if rot13("Rust 2024!") == "Ehfg 2024!" {
        println!("✓ casse et non-lettres préservées");
    } else {
        println!("✗ casse : {}", rot13("Rust 2024!")); ok = false;
    }
    if rot13("").is_empty() {
        println!("✓ vide");
    } else {
        ok = false;
    }
    std::process::exit(if ok { 0 } else { 1 });
}
"#,
        hints: &[
            "Pour un char c : si c.is_ascii_lowercase() → on décale dans l'alphabet. Pareil pour les majuscules avec 'A'.",
            "L'arithmétique sur char n'existe pas directement : passe par u8 (c as u8), calcule ((c - b'a' + 13) % 26) + b'a', puis reconvertis (as char).",
            "s.chars().map(rot13_char).collect() — écris une fonction rot13_char(c: char) -> char.",
        ],
        fiche: "chaines-bases",
    },
    Exo {
        id: "compte_bancaire",
        title: "un compte bancaire (Result métier)",
        difficulty: 4,
        task: "Définis `pub struct CompteBancaire { solde: f64 }` (champ PRIVÉ) avec new(), \
               solde(&self) -> f64, deposer(&mut self, montant: f64) -> Result<(), String> \
               (refuse les montants négatifs ou nuls), et retirer(&mut self, montant: f64) -> \
               Result<(), String> (refuse négatifs, nuls, et les retraits qui dépassent le solde).",
        signature: "pub struct CompteBancaire { ... } + impl CompteBancaire { new, solde, deposer, retirer }",
        starter: "pub struct CompteBancaire {\n    solde: f64,\n}\n\nimpl CompteBancaire {\n    pub fn new() -> Self {\n        todo!(\"à toi\")\n    }\n\n    pub fn solde(&self) -> f64 {\n        todo!(\"à toi\")\n    }\n\n    pub fn deposer(&mut self, montant: f64) -> Result<(), String> {\n        let _ = montant;\n        todo!(\"à toi\")\n    }\n\n    pub fn retirer(&mut self, montant: f64) -> Result<(), String> {\n        let _ = montant;\n        todo!(\"à toi\")\n    }\n}\n",
        harness: r#"fn main() {
    let mut ok = true;
    let mut c = CompteBancaire::new();
    if c.solde() == 0.0 { println!("✓ nouveau compte à 0"); } else { ok = false; }
    if c.deposer(100.0).is_ok() && c.solde() == 100.0 {
        println!("✓ dépôt de 100");
    } else {
        println!("✗ dépôt"); ok = false;
    }
    if c.deposer(-50.0).is_err() && c.solde() == 100.0 {
        println!("✓ dépôt négatif refusé, solde intact");
    } else {
        println!("✗ dépôt négatif"); ok = false;
    }
    if c.retirer(30.0).is_ok() && c.solde() == 70.0 {
        println!("✓ retrait de 30");
    } else {
        println!("✗ retrait valide"); ok = false;
    }
    if c.retirer(1000.0).is_err() && c.solde() == 70.0 {
        println!("✓ découvert refusé, solde intact");
    } else {
        println!("✗ découvert"); ok = false;
    }
    std::process::exit(if ok { 0 } else { 1 });
}
"#,
        hints: &[
            "Le champ privé rend les invariants défendables : on ne peut modifier le solde QUE par tes méthodes.",
            "retirer : if montant <= 0.0 → Err, else if montant > self.solde → Err, else self.solde -= montant; Ok(()).",
            "new() → Self { solde: 0.0 }. solde(&self) → self.solde (lecture seule).",
        ],
        fiche: "erreurs-custom",
    },
];

impl Exo {
    /// Nom du fichier de solution : <id>.rs.
    pub fn solution_name(&self) -> String {
        format!("{}.rs", self.id)
    }
}

pub fn by_id(id: &str) -> Option<&'static Exo> {
    EXOS.iter().find(|e| e.id == id)
}

/// Le harnais complet : import de la solution + tests de l'exo.
pub fn harness_complet(e: &Exo) -> String {
    format!(
        "#[allow(dead_code)]\n#[path = {:?}]\nmod solution;\n#[allow(unused_imports)]\nuse solution::*;\n\n{}",
        e.solution_name(),
        e.harness
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Chaque starter doit AU MOINS compiler avec son harnais (todo!() compile).
    #[test]
    fn harnais_des_exos_compilent() {
        if std::process::Command::new("rustc")
            .arg("--version")
            .output()
            .is_err()
        {
            eprintln!("rustc absent — test sauté");
            return;
        }
        for exo in EXOS {
            let dir = std::env::temp_dir().join(format!("rman-exo-test-{}", exo.id));
            std::fs::create_dir_all(&dir).unwrap();
            let sol = dir.join(exo.solution_name());
            let test = dir.join("_test.rs");
            std::fs::write(&sol, exo.starter).unwrap();
            std::fs::write(&test, harness_complet(exo)).unwrap();
            let out = std::process::Command::new("rustc")
                .args(["--edition", "2021", "-o"])
                .arg(dir.join("_t"))
                .arg(&test)
                .current_dir(&dir)
                .output()
                .unwrap();
            assert!(
                out.status.success(),
                "le harnais de {} doit compiler avec le starter: {}",
                exo.id,
                String::from_utf8_lossy(&out.stderr)
            );
            let _ = std::fs::remove_dir_all(&dir);
        }
    }
}
