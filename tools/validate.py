#!/usr/bin/env python3
"""Validateur de fiches rust-man. Usage: python3 tools/validate.py content/*.toml"""
import sys
import tomllib
import pathlib

REQUIRED = ["id", "title", "category", "difficulty", "synopsis", "description", "example"]
CATEGORIES = {"demarrer", "bases", "ownership", "erreurs", "types", "collections", "outils", "pratique"}

IDS = {
    # demarrer
    "bienvenue", "roadmap", "installer-rust", "lire-la-doc",
    "methode-apprentissage", "premier-programme",
    # bases
    "variables-mutabilite", "types-scalaires", "fonctions",
    "expressions-vs-instructions", "conditions", "boucles", "match",
    "structs", "enums", "tuples", "tableaux", "methodes-impl",
    "modules-use", "chaines-bases",
    # ownership
    "ownership", "emprunts", "emprunts-mutables", "lifetimes", "slices",
    "string-vs-str", "clone-copy", "drop-raii", "rc-refcell",
    # erreurs
    "option", "result", "operateur-question", "panic", "unwrap-expect",
    "erreurs-custom",
    # types
    "generics", "traits", "derive", "traits-communs", "closures",
    "iterateurs", "dyn-trait", "trait-bounds", "types-associes", "conversions",
    # collections
    "vec", "hashmap", "hashset-btreemap", "string-manipulation",
    "iter-avance", "entry-api",
    # outils
    "cargo-bases", "cargo-toml", "tests", "rustfmt", "clippy", "rustdoc",
    "debugging", "crates-io",
    # pratique
    "erreurs-frequentes", "lire-erreur-rustc", "borrow-checker-paix",
    "idiomes-rust", "match-avance", "methodologie-debug", "conseils-cours",
    "structurer-projet",
}


def check(path: pathlib.Path) -> list[str]:
    errors = []
    try:
        data = tomllib.loads(path.read_text(encoding="utf-8"))
    except Exception as e:
        return [f"TOML invalide: {e}"]
    for key in REQUIRED:
        if key not in data:
            errors.append(f"champ manquant: {key}")
    if not errors and data.get("id") != path.stem:
        errors.append(f"id '{data.get('id')}' != nom de fichier '{path.stem}'")
    if data.get("category") not in CATEGORIES:
        errors.append(f"categorie invalide: {data.get('category')!r}")
    d = data.get("difficulty")
    if not isinstance(d, int) or not (1 <= d <= 5):
        errors.append("difficulty doit etre un entier entre 1 et 5")
    if "semaine" in data:
        s = data["semaine"]
        if not (isinstance(s, str) and len(s) == 3 and s.startswith("S") and s[1:].isdigit()):
            errors.append("semaine doit etre une chaine (ex: \"S03\")")
    for key in ("tags", "gotchas", "exercises", "related"):
        if key in data and not (
            isinstance(data[key], list) and all(isinstance(x, str) for x in data[key])
        ):
            errors.append(f"{key} doit etre une liste de chaines")
    for r in data.get("related", []):
        if r not in IDS:
            errors.append(f"related inconnu: {r!r}")
    for key in ("description", "example"):
        v = data.get(key, "")
        if isinstance(v, str) and len(v.strip()) < 80:
            errors.append(f"{key} trop court (<80 caracteres)")
    return errors


def main() -> int:
    bad = 0
    for arg in sys.argv[1:]:
        p = pathlib.Path(arg)
        errs = check(p)
        if errs:
            bad = 1
            for e in errs:
                print(f"{p.name}: {e}")
        else:
            print(f"{p.name}: OK")
    if len(sys.argv) < 2:
        print("usage: validate.py content/*.toml")
        return 2
    return bad


if __name__ == "__main__":
    sys.exit(main())
