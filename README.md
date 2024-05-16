# tori-scrape-lib-rs

[![Rust](https://github.com/MawKKe/tori-scrape-lib-rs/actions/workflows/rust.yml/badge.svg)](https://github.com/MawKKe/tori-scrape-lib-rs/actions/workflows/rust.yml)

**2024-05-15**: tori.fi on uudistanut websivujen rakenteen kovalla kädellä, eli tämä parseri ei enää toimi. En jaksa korjata, pitäkööt tunkkinsa.

---

Kirjasto implementoi parserin joka ottaa tori.fi hakutulossivun (HTML) ja
muuttaa sen rakenteelliseen muotoon. Tätä toiminnallisuutta voidaan käyttää
hyödyksi esimerkiksi hakuvahdin implementointiin.

Huom: tämä kirjasto toimii ns. best effort menetelmällä; koska tori.fi ei tarjoa
mitään varsinaista API:a (ainakaan ilmaiseksi), tämä kirjasto parsii heidän
generoimaa HTML-sekamelskaa sen mukaan miltä HTML on _jollain ajankohdalla_
näyttänyt. Jos tori.fi muuttaa HTML-dokumentin rakennetta, tämä parseri voi
lakata toimimasta. 

Tosin, tämä kirjasto on toteutettu tuo fakta mielessäpitäen; parseri ilmoittaa
selkeästi missä kohtaa meni pieleen, jolloin parseriin tarvittavat muutokset
on helppo päätellä virheviesteistä.

Huom: tämä kirjasto _ei_ implementoi mitään HTTP-pyyntöjen tekemistä tai ajastamista.
Kirjaston päärajapinta on `Parser` luokka, jota käytetään antamalla sille sivuhaun
ajankohta, sekä dokumentin sisältö UTF8-muodossa:

```Rust
    use tori_scrape::{Parser, Item};

    let buf = /* decode HTTP response body to UTF8 */ ;
    let fetch_time = /* ... when the HTTP request was made ... */ ;

    let parser = Parser::new(fetch_time);
    let results: ItemParseResult<Vec<Item>> = parser.parse_from_string(&buf);
```

nyt `results` sisältää joko
- vektorin tori.fi ilmoituksista (`Item`),jotka löytyivät HTML-dokumentista, tai
- virheviestin (`ItemParseError`), joka sisältää tarkennuksen missä parsinta meni mönkään.

Parseri on myös aika suorituskykyinen: 40 ilmoituksen tulossivun parsinta kestää
noin 10 millisekuntia (`--release` moodissa).

Huomaa että tori.fi:n ilmoituksissa olevat aikaleimat ovat aika omituisia ei-standardimaisia ("tänään XX:YY", 
"eilen XX:YY", "29 tam XX:YY", ...); tämä kirjasto
muuntaa ne normaaliksi natiiveiksi aikaleimoiksi, normalisoiden ne UTC:hen, joita on sitten helpompi käyttää jatkoprosessoinnissa.

# Demo

    $ curl <sinun-tori-fi-url> -o my-results.html
    $ cargo run --release --bin parse-demo my-results.html
