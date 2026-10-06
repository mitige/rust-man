# rust-man

Ta documentation Rust dans le terminal — pour apprendre Rust à côté des cours.

`rust-man` affiche une documentation du langage Rust pensée pour accompagner des
cours : 67 fiches courtes et profondes — explication, exemple compilable, pièges
concrets, micro-exercices — accessibles instantanément depuis n'importe quel
terminal ou TTY. Plus 54 questions de quiz, 20 exercices guidés vérifiés pour
de vrai (ton code est compilé et testé), un checker de style, et 19 agents IA.

## Installation (Arch Linux)

```sh
./tools/mkpackage.sh          # construit le paquet
sudo pacman -U package/rust-man-1.0.0-1-x86_64.pkg.tar.zst
```

Ou sans root (binaire dans `~/.local/bin`, déjà dans le PATH de la plupart des shells) :

```sh
cargo install --path . --root ~/.local
```

## Sous Windows

Le zip `rust-man-windows.zip` contient les binaires précompilés dans `windows/` (`rust-man.exe`,
`rust-nano.exe`) : copie-les dans un dossier de ton PATH (ex.
`%USERPROFILE%\bin`) ou lance-les directement. Pour recompiler depuis la
source : installe [rustup](https://rustup.rs) puis `cargo install --path . --root %USERPROFILE%\.cargo`.

L'éditeur rust-nano et le TUI fonctionnent dans le Terminal Windows, PowerShell
et cmd (UTF-8 recommandé : `chcp 65001`).

## Configurer l'IA

Les agents IA (ask, defend, review, plan…) parlent aux modèles via une clé API
compatible OpenAI. Deux façons de la fournir :

```sh
# 1. variable d'environnement
export OPP_API_KEY="ta-clé"          # Windows : setx OPP_API_KEY "ta-clé"

# 2. ou dans ~/.dsh/.credentials.yaml :
OPP_API_KEY: ta-clé
```

`rust-man doctor` vérifie que tout est branché. Sans clé, tout le reste marche
(doc, style, quiz, exos) — seules les fonctions IA sont désactivées.

## rust-nano (éditeur)

`rust-nano <fichier>` (ou `rust-man edit <fichier>`) — éditeur CLI minimaliste :
Tab = 4 espaces, coloration syntaxique, complétion IA en texte fantôme
(Ctrl+Espace, Alt+→ pour accepter), newline final automatique, sauvegarde
Ctrl+S, sortie Ctrl+Q.

## Utilisation

```
rust-man                  navigateur interactif (TUI)
rust-man ownership        affiche la fiche, comme man
rust-man -l [catégorie]   liste les fiches · -s <mot> recherche · --random au hasard

── apprendre ──
rust-man apprendre <sujet>  multi-angle : lis → teste → pratique → explique
rust-man exo [<id>] [--test] [--indice n]   exercices guidés, code vérifié (rustc)
rust-man defend <sujet>     oral blanc (le Coach t'évalue)
rust-man quiz [cat]         quiz · rust-man morning  routine du matin

── coder ──
rust-man new <nom> [-f "fn aire(r: f64) -> f64"]   squelette .rs qui compile
rust-man new-projet <nom>   projet cargo complet (lib.rs + test + README)
rust-man edit <fichier>     l'éditeur rust-nano (complétion IA)

── vérifier ──
rust-man check <fichier|dir>  style Rust (tabs, 100 col, snake_case, unwrap…)
rust-man fix <fichier|dir>    corrige les fautes auto-corrigibles
rust-man explain R-UNWRAP     explique une règle de style
rust-man test [dir]           cargo test, ou compile + lance main.rs
rust-man watch [dir]          re-vérifie le style à chaque sauvegarde
rust-man review [dir]         revue IA complète du projet avant un rendu

── projets ──
rust-man rush <sujet> [-n nom]  analyse l'énoncé + scaffold le projet, d'un coup
rust-man analyse <sujet|- >     décompose un énoncé (livrables, contraintes, plan)
rust-man plan "<mission>"       l'orchestrateur construit ton plan d'attaque
rust-man serve [-p 8080]        sert le dossier en HTTP (cargo doc, par ex.)
rust-man push "msg"             le rituel de livraison (style → add → commit → push)

── divers ──
rust-man next                   la SEULE chose à faire maintenant
rust-man recap                  bilan du soir + la suite pour demain
rust-man journal "…"            ta réflexion d'apprentissage (écrite)
rust-man note <fiche> "…"       ta compréhension, avec tes mots
rust-man bloque "<problème>"    bloqué ? routé vers la bonne aide
rust-man apropos <mot>          cherche partout (fiches, exos, quiz, agents)
rust-man focus [min]            minuteur pomodoro
rust-man progress               ton tableau de bord (streak, semaine, focus)
rust-man doctor                 vérifie ton environnement Rust
rust-man ask <agent> "<q>"      les agents IA (19 : binômes, intervenant, experts…)
```

### Dans le navigateur

| Touche | Action |
|---|---|
| `j` `k` / flèches | naviguer, défiler |
| taper du texte | recherche floue instantanée |
| `Enter` | ouvrir la fiche |
| `h` / `Échap` | retour à la liste |
| `n` / `p` | fiche liée suivante / précédente |
| `b` | retour historique |
| `espace` / `u` | page suivante / précédente |
| `g` / `G` | haut / bas du document |
| `?` | aide |
| `q` | quitter |

## Contenu

- **Démarrer** : bienvenue, roadmap sur 8 semaines, installer rustup, lire la doc,
  premier programme, méthode d'apprentissage
- **Les bases** : variables & mutabilité, types scalaires, fonctions, expressions
  vs instructions, conditions, boucles, match, structs, enums, tuples, tableaux,
  méthodes & impl, modules & use, chaînes
- **Ownership & emprunts** (le cœur) : ownership, emprunts, emprunts mutables,
  lifetimes, slices, String vs &str, Clone vs Copy, Drop & RAII, Rc & RefCell
- **Gestion d'erreurs** : Option, Result, l'opérateur ?, panic!, unwrap/expect,
  erreurs personnalisées
- **Generics, traits & closures** : traits, derive, la trousse std, generics,
  bornes, closures, itérateurs, conversions, dyn, types associés
- **Collections & String** : Vec, HashMap, HashSet & BTreeMap, manipulation de
  chaînes, itérateurs avancés, entry API
- **Cargo & outils** : cargo, Cargo.toml, tests, rustfmt, clippy, rustdoc,
  debugging, crates.io
- **Méthode & pratique** : erreurs fréquentes, lire une erreur rustc, la paix
  avec le borrow checker, idiomes, match avancé, méthodologie de debug,
  articuler avec les cours, structurer un projet

Chaque fiche est un fichier TOML dans `content/`, validé et embarqué à la
compilation : le binaire est autonome, aucune dépendance à l'exécution.

## Développement

```sh
cargo test                                 # tests unitaires (dont « les harnais d'exos compilent »)
python3 tools/validate.py content/*.toml   # valide les fiches
cargo build --release
```
