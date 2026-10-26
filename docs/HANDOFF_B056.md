# Handoff B-056 · BUs do exterior no ar antes do `cs.json` (25/10/2026, madrugada do 2º turno)

Estado: **trabalho em andamento, não compilado nem testado.** A sessão do Claude perdeu o shell no meio do passo 1 da
regra de bugs do [CLAUDE.md](../CLAUDE.md) e publicou este handoff pela API do GitHub. **As mudanças de código não
foram commitadas: estão no patch da última seção** (aplicar com `git apply`). Este commit não sobe a versão (não deu
para rodar `scripts/versao.sh`); o commit da correção sobe para `0.9.21`.

## Sintoma (conferido no site do TSE, 06:01–06:12 UTC = 03:01–03:12 de Brasília)

- `zz-p003221-cs.json`: `last-modified: Fri, 09 Oct 2026 20:32:39 GMT`, `dg`/`hg` = 09/10/2026 17:30:13, **nenhuma** das
  2.715 seções com `da`/`ha` (1.364 delas com `nsp`, agregadas). As 27 UFs do Brasil também sem disponibilização
  (esperado: antes das 17h).
- Mesmo assim o `aux.json` de Wellington já estava no ar:
  `https://resultados.tse.jus.br/oficial/ele2026/arquivo-urna/3221/dados/zz/30805/0001/1088/p003221-zz-m30805-z0001-s1088-aux.json`
  → `"st":"Recebida"`, `dr`/`hr` 25/10/2026 01:51:38, BU `"Recebido"`.
- O BU respondeu 200 (791 bytes):
  `…/1088/36576c557a394d72394530736f30676e3630384446537969547059394474785771315136714d46506a48733d/o03221zz3080500011088-bu.dat`.
- No 1º turno (3220, `cs.json` final), Wellington tem disponibilização 04/10 01:49:24 e Camberra 03:40:32.
- O `uv-servico` (`acompanhar`) só pede `aux.json` de seção com disponibilização no `cs.json`, e a sondagem
  (`Motor::sondar`) só roda depois do primeiro BU aplicado e quando faltam poucas seções: **não vê nenhum BU do
  exterior enquanto o TSE não atualizar o `cs.json`** (não se sabe quando ele atualiza; talvez só a partir das 17h).

Requisições feitas pela sessão: 1 `ele-c.json`, 31 `cs.json` do 3221 (1 req/s), 1 `cs.json` do 3220, 1 `aux.json`,
1 BU.

## Linha para o `docs/BUGS.md` (não está no commit; vai no patch abaixo)

`| B-056 | 25/10/2026 | Luciano ("no primeiro turno a zz já divulgava às 1h") e Claude (conferência no site do TSE) |
uv-servico, acompanhamento dos cs.json, exterior do 3221 | (sintoma acima) | … | imediato (Luciano, 25/10) |
servico::testes::exterior_sondado_antes_do_cs_json | 0.9.21 |` — falta também a seção "### B-056" com as saídas
antes/depois no fim do arquivo.

## O que o patch faz

1. `aquisicao/src/servico.rs`:
   - `enum Sonda { Nao, SeResolvidas, Antecipada }`, `fn quando_sondar(zz, com_nsp, n, ap, publicadas: Option<u64>,
     algo_apurado, domingo) -> Sonda` e `fn domingo(t)` (junto das constantes `SONDA_*`). **`quando_sondar` ainda tem o
     comportamento antigo** (nunca devolve `Antecipada`): é a versão "antes", para ver o teste falhar.
   - `Motor::sondar` usa `quando_sondar` (o `ultimo_bu_site.is_none()` do topo vira o `algo_apurado`); a lista de
     sondagem ganha prioridade `(u8, i64, ChaveUf)`: as seções dos municípios que já têm BU primeiro; a exigência
     "publicadas resolvidas" só vale em `Sonda::SeResolvidas`.
   - Doc do `sondar` e do `acompanhar` com a regra nova.
   - Teste novo `testes::exterior_sondado_antes_do_cs_json` (números do 3221 de hoje).
2. `aquisicao/src/tse.rs:12`: comentário do nome do BU corrigido (`o<pleito:05>…`, não `o0<pleito:05>…`).
3. `docs/BUGS.md`: a linha do B-056.

## Para terminar (ordem da regra de bugs)

1. `git apply` do patch; `cargo test --release -p uv-aquisicao --lib exterior_sondado_antes`. Corrigir o que não
   compilar; **ver falhar** na asserção "B-056: o exterior não é sondado antes do cs.json" e colar a saída no
   `docs/BUGS.md`.
2. A correção, em `quando_sondar`, antes da regra antiga (e tirar o `let _ = (publicadas, domingo);`):
   ```rust
   if zz && com_nsp && domingo && publicadas == Some(0) && ap < n { return Sonda::Antecipada; }
   ```
3. Teste de novo (passa) e a suíte: `cargo test --release -p urna-velox` e `-p uv-aquisicao`; com dados reais,
   `UV_DADOS=… cargo test --release -p uv-aquisicao -- --ignored --test-threads 1`. Atenção ao
   `servico::nao_instaladas_exterior` (exige `aux_404 < 100`): com o simulador começando antes do primeiro BU do ZZ, a
   sondagem antecipada pode gerar mais 404; se falhar só por isso, decidir (limite do ensaio ou começo do simulado).
4. `scripts/versao.sh bump patch` (0.9.21), CHANGELOG com o teste antes/depois. Não mexe em totalização: sem
   conferência contra os oficiais.
5. Implantar nos nós (`50_atualizar.sh`) só com o seu acordo (é coleta durante a apuração).

## Carga esperada contra o TSE

ZZ: 1.351 seções sondáveis (2.715 − 1.364 agregadas), ≤ 2 req/s (`SONDA_POR_RODADA`), cada uma no máximo a cada 300 s:
uma volta completa em ~11 min, quase tudo 404 até o horário de cada posto. Na federação, cada nó só vai ao TSE pelas
seções da sua partição (as outras sondagens são descartadas sem requisição). O 429 por 404 seguidos foi visto a
~70 req/s (12/10). Só aos domingos e só enquanto o `cs.json` do ZZ não listar nenhuma seção; `--sem-sondagem` desliga.

Em aberto: o `domingo` usa o relógio do site (`Date` das respostas, hora de Brasília); antes da primeira resposta ele
vale 0 (quinta-feira) e nada é antecipado. Suplementar fora de domingo não é coberta. Quando o `cs.json` do ZZ listar
a primeira seção, volta a regra de 16/10 (só quando faltam poucas).

## Patch (sobre `e9ebc61`, 0.9.20)

```diff
--- a/aquisicao/src/tse.rs
+++ b/aquisicao/src/tse.rs
@@ -9,7 +9,7 @@
 //! <base>/<amb>/<ciclo>/arquivo-urna/<pleito>/config/<uf>/<uf>-p<pleito:06>-cs.json   seções da UF
 //! <base>/<amb>/<ciclo>/arquivo-urna/<pleito>/dados/<uf>/<mun:05>/<zona:04>/<secao:04>/
 //!        p<pleito:06>-<uf>-m<mun:05>-z<zona:04>-s<secao:04>-aux.json      arquivos da seção
-//!        <hash>/o0<pleito:05><uf><mun:05><zona:04><secao:04>-bu.dat       BU (envelope BER)
+//!        <hash>/o<pleito:05><uf><mun:05><zona:04><secao:04>-bu.dat        BU (envelope BER)
 //! ```
 //! `<hash>` é o SHA-256 (base64, e depois em hexadecimal) do arquivo de assinaturas `…-vota.vsc`.
 
--- a/aquisicao/src/servico.rs
+++ b/aquisicao/src/servico.rs
@@ -345,6 +345,29 @@ pub const SONDA_ABR: usize = 1500;
 pub const SONDA_POR_RODADA: usize = 2;
 /// ... numa UF do Brasil, só quando faltam no máximo estas seções para o `s.ts` (no exterior, 5 % ou 50)
 pub const SONDA_FALTAM_UF: u64 = 50;
+
+/// Se as seções sem disponibilização de uma UF entram na sondagem ([`Motor::sondar`]).
+#[derive(Clone, Copy, Debug, PartialEq, Eq)]
+enum Sonda {
+    Nao,
+    /// só quando toda seção com disponibilização no `cs.json` da UF já estiver resolvida
+    SeResolvidas,
+    /// já, sem esperar a disponibilização no `cs.json`
+    Antecipada,
+}
+
+/// Decide a sondagem de uma UF: `n` seções esperadas, `ap` apuradas, `publicadas` com disponibilização no último
+/// `cs.json` (`None`: ainda não lido), `algo_apurado`: algum BU já aplicado, `domingo`: o relógio do site está num
+/// domingo.
+fn quando_sondar(zz: bool, com_nsp: bool, n: u64, ap: u64, publicadas: Option<u64>, algo_apurado: bool, domingo: bool) -> Sonda {
+    let _ = (publicadas, domingo);
+    if !algo_apurado || (!zz && !com_nsp) || ap == 0 || ap >= n { return Sonda::Nao; }
+    let limite = if zz { (n / 20).max(50) } else { SONDA_FALTAM_UF };
+    if n - ap > limite { Sonda::Nao } else { Sonda::SeResolvidas }
+}
+
+/// O instante (segundos civis de Brasília, como o `relogio_site`) cai num domingo (01/01/1970 foi quinta-feira).
+fn domingo(t: i64) -> bool { (t.div_euclid(86400) + 4).rem_euclid(7) == 0 }
 
 // ------------------------------------------------------------------ diário de seções
 
@@ -447,7 +470,7 @@ fn reenfileirar(com: &Arc<Comum>, tx: &FilaTx, mut t: Tarefa, base: f64, max_exp: u32) {
 }
 
 /// Relê os `cs.json` das UFs a cada `intervalo` segundos e enfileira cada seção nova ou com disponibilização
-/// alterada. Seção sem disponibilização não gera requisição.
+/// alterada. Seção sem disponibilização não gera requisição aqui (a sondagem, [`Motor::sondar`], cuida das exceções).
 pub async fn acompanhar(com: Arc<Comum>, ufs: Vec<String>, mut conhecidas: HashMap<ChaveUf, i64>, tx: FilaTx) {
     let mut etags: HashMap<String, String> = HashMap::new();
     let mut ausentes: HashSet<String> = HashSet::new();
@@ -1573,58 +1596,79 @@ impl Motor<'_> {
     /// fora as agregadas (`nsp` no `cs.json`, que não têm `aux.json`), cada uma de novo a cada [`SONDA_REPETIR_S`]
     /// (hora do site), no máximo [`SONDA_POR_RODADA`] por segundo e [`SONDA_ABR`] seções por UF. "Nao instalada"
     /// conta; 404 ou sem BU, a seção continua esperada. Sem `nsp` no `cs.json` da UF (arquivo antigo), o comportamento
-    /// de antes: só o exterior, uma vez cada, 404 = agregada. É a única exceção à regra "seção sem disponibilização não
-    /// gera requisição". Antes da eleição (nada apurado) não sonda nada; `--sem-sondagem` desliga.
+    /// de antes: só o exterior, uma vez cada, 404 = agregada. Antes da eleição (nada apurado) não sonda nada.
+    ///
+    /// Exterior antes do `cs.json` (B-056, 25/10/2026): o TSE põe no ar o `aux.json` e o BU das seções do exterior
+    /// horas antes de pôr a disponibilização no `cs.json` (3221: Wellington recebido às 01:51:38, `cs.json` ainda de
+    /// 09/10 às 03:07). Num domingo (relógio do site), enquanto o `cs.json` do ZZ (com `nsp`) não listar nenhuma seção,
+    /// as seções do ZZ sem BU são sondadas já, nas mesmas regras (fora as agregadas, ≤ [`SONDA_POR_RODADA`] por
+    /// segundo, cada uma a cada [`SONDA_REPETIR_S`]), primeiro as dos municípios que já têm BU. São as duas exceções
+    /// à regra "seção sem disponibilização não gera requisição"; `--sem-sondagem` desliga as duas.
     fn sondar(&mut self) {
-        if !self.com.cfg.sondar || self.encerrada || self.ultimo_bu_site.is_none() { return; }
+        if !self.com.cfg.sondar || self.encerrada { return; }
+        let algo_apurado = self.ultimo_bu_site.is_some();
         let com = self.com;
         let agora = com.relogio_site.load(Relaxed);
         let publicadas = com.publicadas_cs.lock().unwrap().clone();
         let agregadas = com.agregadas_cs.lock().unwrap();
         let vazio = HashSet::new();
-        // (última sondagem, seção): as nunca sondadas primeiro, depois as mais antigas
-        let mut lista: Vec<(i64, ChaveUf)> = vec![];
+        // (prioridade, última sondagem, seção): município com BU primeiro, depois as nunca sondadas, depois as mais antigas
+        let mut lista: Vec<(u8, i64, ChaveUf)> = vec![];
         for (abr, n) in &self.esperado {
             let Abrangencia::Uf(u) = abr else { continue };
             let n = self.esperadas(abr, *n);
             let uf_s = String::from_utf8_lossy(u).into_owned();
             let ag = agregadas.get(&uf_s).unwrap_or(&vazio);
             let com_nsp = !ag.is_empty();
             let zz = u == b"zz";
-            if !zz && !com_nsp { continue; }
             let ap = self.pubr.secoes(self.est, abr).1;
-            if ap == 0 || ap >= n { continue; }
-            let limite = if zz { (n / 20).max(50) } else { SONDA_FALTAM_UF };
-            if n - ap > limite { continue; }
-            // toda seção com disponibilização da UF já resolvida (as que faltam são as sem disponibilização)
+            let pub_uf = publicadas.get(&uf_s).copied();
+            let quando = quando_sondar(zz, com_nsp, n, ap, pub_uf, algo_apurado, domingo(agora));
+            if quando == Sonda::Nao { continue; }
             let mut resolvidas = 0u64;
+            // (município com BU primeiro, seção): na antecipada, as seções do mesmo posto saem juntas
             let mut v = vec![];
             for m in self.munis_de(abr) {
+                let mut com_bu = false;
+                let ini = v.len();
                 for &k in &self.secoes_muni[m] {
                     let ku = (*u, k);
                     match self.secoes.get(&ku) {
-                        Some(Situ::Bu { .. } | Situ::NaoInstalada) => resolvidas += 1,
+                        Some(Situ::Bu { .. }) => { resolvidas += 1; com_bu = true; }
+                        Some(Situ::NaoInstalada) => resolvidas += 1,
                         Some(Situ::Agregada) => {}
-                        None => if !ag.contains(&k) && !self.pendentes.contains_key(&ku) { v.push(ku); },
+                        None => if !ag.contains(&k) && !self.pendentes.contains_key(&ku) { v.push((1u8, ku)); },
                     }
                 }
+                if com_bu { for x in &mut v[ini..] { x.0 = 0; } }
             }
-            if resolvidas < publicadas.get(&uf_s).copied().unwrap_or(u64::MAX) || v.is_empty() { continue; }
+            // fora da antecipada: toda seção com disponibilização da UF já resolvida (as que faltam são as sem
+            // disponibilização)
+            if quando == Sonda::SeResolvidas && resolvidas < pub_uf.unwrap_or(u64::MAX) { continue; }
+            if v.is_empty() { continue; }
             if v.len() > SONDA_ABR {
                 if self.sonda_recusada.insert(abr.codigo()) {
                     com.alerta("sondagem_limitada", &format!("{}: {} seções sem disponibilização; sondagem recusada (limite de {SONDA_ABR} por UF) — confira as não instaladas pelo oficial", abr.codigo(), v.len()));
                 }
                 continue;
             }
-            for ku in v {
+            for (prio, ku) in v {
                 match self.sondadas.get(&ku) {
-                    None => lista.push((i64::MIN, ku)),
-                    Some(&t) if com_nsp && agora - t >= SONDA_REPETIR_S => lista.push((t, ku)),
+                    None => lista.push((prio, i64::MIN, ku)),
+                    Some(&t) if com_nsp && agora - t >= SONDA_REPETIR_S => lista.push((prio, t, ku)),
                     _ => {}
                 }
             }
         }
         drop(agregadas);
         if lista.is_empty() { return; }
         lista.sort_unstable();
         lista.truncate(SONDA_POR_RODADA);
         let mut pedido = vec![];
-        for (_, ku) in lista {
+        for (_, _, ku) in lista {
             self.sondadas.insert(ku, agora);
             pedido.push(Secao { uf: ku.0, municipio: (ku.1 >> 32) as u32, zona: (ku.1 >> 16) as u16, secao: ku.1 as u16, disponivel: String::new() });
         }
@@ -2387,6 +2431,31 @@ mod testes {
         let (a2, b2) = (sec(b"ba", 5, "04/10/2026 18:00:00"), sec(b"am", 9, "04/10/2026 18:00:00"));
         assert!(chave_ordem(&vazio, &b2) < chave_ordem(&vazio, &a2));
     }
+    /// B-056 (25/10/2026, 2º turno, pleito 3221): às 06:07 UTC o `aux.json` de Wellington (ZZ 30805/0001/1088) já dizia
+    /// BU "Recebido" às 01:51:38 e o BU estava no ar, mas o `zz-p003221-cs.json` ainda era o de 09/10 (nenhuma das
+    /// 2.715 seções com disponibilização). O exterior tem de ser sondado já, sem esperar o `cs.json` nem um primeiro BU.
+    #[test]
+    fn exterior_sondado_antes_do_cs_json() {
+        let t = |s: &str| urna_velox::metricas::segundos(s).unwrap();
+        assert!(domingo(t("25/10/2026 03:07:00")) && domingo(t("04/10/2026 00:00:00")) && domingo(t("27/10/2024 23:59:59")));
+        assert!(!domingo(t("24/10/2026 23:59:59")) && !domingo(t("26/10/2026 00:00:00")));
+        // ZZ do 3221: 1.351 seções esperadas (2.715 − 1.364 agregadas), nada apurado, cs.json lido sem disponibilização
+        assert_eq!(quando_sondar(true, true, 1351, 0, Some(0), false, true), Sonda::Antecipada, "B-056: o exterior não é sondado antes do cs.json");
+        // achados os primeiros BUs pela sondagem, segue antecipada enquanto o cs.json não listar nada
+        assert_eq!(quando_sondar(true, true, 1351, 5, Some(0), true, true), Sonda::Antecipada);
+        // fora do domingo, sem o cs.json lido ou sem as agregadas (nsp) no cs.json: nada antecipado
+        assert_eq!(quando_sondar(true, true, 1351, 0, Some(0), false, false), Sonda::Nao);
+        assert_eq!(quando_sondar(true, true, 1351, 0, None, false, true), Sonda::Nao);
+        assert_eq!(quando_sondar(true, false, 1351, 0, Some(0), false, true), Sonda::Nao);
+        // UF do Brasil: nunca antecipada
+        assert_eq!(quando_sondar(false, true, 38738, 0, Some(0), false, true), Sonda::Nao);
+        // com o cs.json listando seções, a regra de antes (16/10): só quando faltam poucas e as publicadas resolvidas
+        assert_eq!(quando_sondar(true, true, 1351, 5, Some(5), true, true), Sonda::Nao);
+        assert_eq!(quando_sondar(true, true, 1351, 1310, Some(1310), true, true), Sonda::SeResolvidas);
+        assert_eq!(quando_sondar(false, true, 38738, 38700, Some(38700), true, true), Sonda::SeResolvidas);
+        assert_eq!(quando_sondar(false, true, 38738, 38738, Some(38738), true, true), Sonda::Nao);
+    }
     #[test]
     fn painel_desenha() {
         assert_eq!((mil(0), mil(999), mil(1000), mil(1234567)), ("0".into(), "999".into(), "1.000".into(), "1.234.567".into()));
```

Os números de linha dos `@@` são aproximados (escritos à mão): se o `git apply` recusar, use `git apply --recount`
ou `patch -p1 --fuzz=3`, ou aplique os trechos à mão. Uma cópia do `servico.rs`, do `tse.rs` e do `BUGS.md` com as
mudanças ficou no container da sessão do Claude (`/home/user/urna-velox`, sem commit).
