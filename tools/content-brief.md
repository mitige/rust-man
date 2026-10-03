# Brief contenu rust-man — à lire ENTIÈREMENT avant d'écrire

`rust-man` est un outil terminal d'apprentissage du Rust pour un étudiant qui suit
des cours de Rust à l'université et veut un compagnon de révision dans son terminal.
C'est le pendant de `rust-man` (qui visait la piscine Epitech en C). **Aucune référence
à Epitech, la piscine, la moulinette ou la Norme** : le contexte est « j'apprends
Rust à côté de mes cours ».

## Schéma TOML d'une fiche (OBLIGATOIRE, tous les champs requis sauf mention)

```toml
id = "ownership"                       # EXACTEMENT le nom du fichier sans .toml
title = "ownership — qui possède quoi" # format : « mot-clé — accroche courte »
category = "ownership"                 # une des 8 catégories ci-dessous
tags = ["move", "possession", "drop"]  # 3-6 tags, minuscules
difficulty = 3                         # 1 (trivial) à 5 (avancé)
semaine = "S03"                        # optionnel : S01..S08 (parcours de 8 semaines)
synopsis = "let a = b;  // b est DÉPLACÉE dans a — b n'existe plus"
description = '''...'''                # markdown, voir structure ci-dessous
example = '''...'''                    # programme Rust COMPLET et compilable
gotchas = [ "...", "..." ]             # 4 à 6 pièges concrets, une phrase chacun
exercises = [ "...", "..." ]           # 2 à 3 micro-exercices à faire soi-même
related = ["emprunts", "clone-copy"]   # 3 à 5 ids de fiches EXISTANTES (liste maître)
```

## Catégories (valeur exacte du champ `category`)

- `demarrer` — prise en main, méthode, installation
- `bases` — syntaxe et constructions de base du langage
- `ownership` — possession, emprunts, durées de vie (le cœur de Rust)
- `erreurs` — Option, Result, panic, erreurs custom
- `types` — generics, traits, closures, itérateurs
- `collections` — Vec, HashMap, String, iterators avancés
- `outils` — cargo, rustfmt, clippy, tests, docs
- `pratique` — méthode, idiomes, lecture d'erreurs, organisation

## Structure du champ `description` (markdown)

Trois sections, dans cet ordre, titres EXACTS :

```
## En une phrase

<la définition viscérale, en une phrase, avec le point clé en **gras**>

## En pratique

<le corps : 15-40 lignes. Règles numérotées, mini-exemples ```rust, schémas ASCII
quand ça aide (mémoire, emprunts). Concret, jamais de théorie gratuite.>

## À retenir

- <3 à 6 puces, une idée chacune, la version « flashcard » de la fiche>
```

## Ton (crucial)

- Français, **tutoiement**, direct, chaleureux mais exigeant sur la précision.
- Le lecteur est un étudiant qui DÉBUTE en Rust mais sait déjà programmer (il suit
  des cours). Pas de « qu'est-ce qu'une variable » — plutôt « en Rust, une variable
  est immuable par défaut, et voilà pourquoi c'est génial ».
- Compare au C/Python/JS quand ça éclaire, sans jargon inutile.
- Chaque affirmation doit être **techniquement exacte** pour Rust stable édition 2021.
- Les exemples de code dans `description` sont courts (3-10 lignes).

## Le champ `example` (crucial)

- Programme Rust **complet, autonome, qui compile** avec `rustc --edition 2021 fichier.rs`.
- Toujours un `fn main()`. Pas de dépendance externe, std uniquement.
- 15-45 lignes, commenté sobrement en français (commentaires `//` courts).
- Il doit tourner et produire une sortie sensible (println!) sauf si le sujet
  l'interdit (ex: une fiche sur panic! peut montrer un panic rattrapé).
- Si tu montres volontairement du code qui NE compile PAS (pédagogique), mets-le
  dans `description` en bloc ```rust accompagné de « ne compile pas », jamais
  dans `example`.

## Vérifications avant de livrer chaque fiche

1. `id` == nom du fichier (sans .toml), en kebab-case.
2. `category` dans la liste des 8.
3. `difficulty` entre 1 et 5.
4. Tous les ids de `related` existent dans la LISTE MAÎTRE ci-dessous (rien d'autre).
5. Le TOML est valide : chaînes multi-lignes entre `'''...'''`, pas de `'''` dans
   le contenu. Apostrophes typographiques OK.
6. L'exemple compile mentalement : types corrects, emprunts valides, imports présents.

## LISTE MAÎTRE des ids de fiches (seuls ids valides pour `related`)

demarrer : bienvenue, roadmap, installer-rust, lire-la-doc, methode-apprentissage, premier-programme
bases : variables-mutabilite, types-scalaires, fonctions, expressions-vs-instructions, conditions, boucles, match, structs, enums, tuples, tableaux, methodes-impl, modules-use, chaines-bases
ownership : ownership, emprunts, emprunts-mutables, lifetimes, slices, string-vs-str, clone-copy, drop-raii, rc-refcell
erreurs : option, result, operateur-question, panic, unwrap-expect, erreurs-custom
types : generics, traits, derive, traits-communs, closures, iterateurs, dyn-trait, trait-bounds, types-associes, conversions
collections : vec, hashmap, hashset-btreemap, string-manipulation, iter-avance, entry-api
outils : cargo-bases, cargo-toml, tests, rustfmt, clippy, rustdoc, debugging, crates-io
pratique : erreurs-frequentes, lire-erreur-rustc, borrow-checker-paix, idiomes-rust, match-avance, methodologie-debug, conseils-cours, structurer-projet

## Schéma TOML d'une question de quiz (pour les batches quiz)

```toml
id = "q-move-basique"                  # commence par q-, == nom du fichier
category = "ownership"                 # une des 8 catégories
difficulty = 2                         # 1..5
question = "Après `let b = a;` (a: String), que peut-on faire ?"
choices = ["Utiliser a et b", "Utiliser b seulement", "Utiliser a seulement", "Rien"]
answer = 1                             # index 0-based du bon choix
explanation = '''...'''                # 2-5 phrases : pourquoi, et la règle générale
related = ["ownership"]                # 1 à 3 ids de fiches de la liste maître
```

Règles quiz : 4 choix (3 minimum, vise 4), UNE seule bonne réponse, les mauvaises
réponses sont des misconceptions réelles de débutant (pas des leurres absurdes).
Varie la position de la bonne réponse d'une question à l'autre.
