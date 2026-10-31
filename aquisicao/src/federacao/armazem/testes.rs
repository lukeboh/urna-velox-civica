use super::*;

fn temp(nome: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("uv-armazem-{}-{nome}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    d
}

#[test]
fn gravar_ler_etag_listar() {
    let d = temp("basico");
    let a = ArmazemLocal::novo(&d).unwrap();
    assert_eq!(a.ler("nos/a.json", None).unwrap(), Lido::NaoExiste);
    let v = a.gravar("nos/a.json", b"{\"x\":1}").unwrap();
    assert_eq!(v.etag.len(), 64);
    match a.ler("nos/a.json", None).unwrap() { Lido::Novo(o) => assert_eq!(o.bytes, b"{\"x\":1}"), x => panic!("{x:?}") }
    assert_eq!(a.ler("nos/a.json", Some(&v.etag)).unwrap(), Lido::NaoMudou);
    let v2 = a.gravar("nos/a.json", b"{\"x\":2}").unwrap();
    assert_ne!(v.etag, v2.etag);
    assert!(matches!(a.ler("nos/a.json", Some(&v.etag)).unwrap(), Lido::Novo(_)));
    a.gravar("nos/b.json", b"b").unwrap();
    a.gravar("bus/no-a/000001.uvpk", b"pk").unwrap();
    let nos: Vec<String> = a.listar("nos/").unwrap().into_iter().map(|(k, _)| k).collect();
    assert_eq!(nos, ["nos/a.json", "nos/b.json"]);
    assert_eq!(a.listar("").unwrap().len(), 3);
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn chaves_recusadas() {
    let a = ArmazemLocal::novo(temp("chaves")).unwrap();
    for c in ["", "/x", "x/", "a/../b", "a//b", "a b", "../fora", "nos/.", "c:\\x"] {
        assert!(a.gravar(c, b"x").is_err(), "{c}");
    }
}
