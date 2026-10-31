use super::*;
#[test]
fn truncado_nao_entra_em_panico() {
    let bu = [0x30u8, 0x82, 0xff, 0xff, 0x30];
    assert!(ler_bu(&bu, &mut Texto(String::new())).is_err());
    assert!(ler_bu(&[], &mut Texto(String::new())).is_err());
}
