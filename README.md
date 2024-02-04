# tori-scrape-lib-rs

[![Rust](https://github.com/MawKKe/tori-scrape-lib-rs/actions/workflows/rust.yml/badge.svg)](https://github.com/MawKKe/tori-scrape-lib-rs/actions/workflows/rust.yml)

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
    use tori_scrape_lib::{Parser, Item};

    let buf = /* decode HTTP response body to UTF8 */ ;
    let fetch_time = /* ... when the HTTP request was made ... */ ;

    let parser = Parser::new(fetch_time);
    let result: Vec<Item> = parser.parse_from_string(&buf).unwrap();
```

nyt `results` sisältää vektorin tori.fi ilmoituksista jotka löytyivät HTML-dokumentista (jos löytyivät...).
