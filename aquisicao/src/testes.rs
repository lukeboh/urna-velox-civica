use super::*;
#[test]
fn datas_http() {
    let t = urna_velox::metricas::segundos("27/10/2024 17:01:02").unwrap();
    let h = data_http_de_brasilia(t);
    assert_eq!(h, "Sun, 27 Oct 2024 20:01:02 GMT");
    assert_eq!(data_http_brasilia(&h), Some(t));
    assert_eq!(texto_hora(t), "27/10/2024 17:01:02");
    assert!(agora_brasilia() > urna_velox::metricas::segundos("01/01/2026 00:00:00").unwrap());
}
