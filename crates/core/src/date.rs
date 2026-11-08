//! Datas de um dia só, sem relógio: `AAAA-MM-DD` que existe no calendário.
//! O dia de hoje entra por parâmetro em todo lugar (o binário lê o relógio ou
//! `FREITASK_TODAY` uma vez e passa para cá).

use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Date {
    pub ano: i32,
    pub mes: u32,
    pub dia: u32,
}

fn bissexto(ano: i32) -> bool {
    (ano % 4 == 0 && ano % 100 != 0) || ano % 400 == 0
}

fn dias_no_mes(ano: i32, mes: u32) -> u32 {
    match mes {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if bissexto(ano) => 29,
        2 => 28,
        _ => 0,
    }
}

impl Date {
    pub fn new(ano: i32, mes: u32, dia: u32) -> Option<Date> {
        if !(1..=12).contains(&mes) || dia < 1 || dia > dias_no_mes(ano, mes) {
            return None;
        }
        Some(Date { ano, mes, dia })
    }

    /// Só `AAAA-MM-DD` de um dia que existe, exatamente: sem espaço, sem hora,
    /// sem dígito faltando. Qualquer outra coisa é `None`, e quem chama decide
    /// (o `doctor` acusa; a adiada não some).
    pub fn parse(s: &str) -> Option<Date> {
        let b = s.as_bytes();
        if b.len() != 10 || b[4] != b'-' || b[7] != b'-' {
            return None;
        }
        let digitos = |faixa: std::ops::Range<usize>| -> Option<u32> {
            let t = &s[faixa];
            if !t.bytes().all(|c| c.is_ascii_digit()) {
                return None;
            }
            t.parse().ok()
        };
        let ano = digitos(0..4)?;
        let mes = digitos(5..7)?;
        let dia = digitos(8..10)?;
        Date::new(ano as i32, mes, dia)
    }

    /// Dias desde 1970-01-01 (algoritmo civil de Howard Hinnant).
    pub fn dias_civis(self) -> i64 {
        let y = i64::from(self.ano) - i64::from(self.mes <= 2);
        let era = if y >= 0 { y } else { y - 399 } / 400;
        let yoe = y - era * 400;
        let m = i64::from(self.mes);
        let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + i64::from(self.dia) - 1;
        let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
        era * 146_097 + doe - 719_468
    }

    /// `other - self` em dias.
    pub fn dias_ate(self, other: Date) -> i64 {
        other.dias_civis() - self.dias_civis()
    }

    /// Nome do dia da semana em inglês, como `os.date("%A")` sob `LANG=C`.
    pub fn weekday(self) -> &'static str {
        // 1970-01-01 foi quinta-feira.
        match (self.dias_civis() + 4).rem_euclid(7) {
            0 => "Sunday",
            1 => "Monday",
            2 => "Tuesday",
            3 => "Wednesday",
            4 => "Thursday",
            5 => "Friday",
            _ => "Saturday",
        }
    }

    /// `DD/MM/AA`, o formato do título do painel.
    pub fn curto(self) -> String {
        format!(
            "{:02}/{:02}/{:02}",
            self.dia,
            self.mes,
            self.ano.rem_euclid(100)
        )
    }

    /// `DD/MM`, o formato do prazo na TUI.
    pub fn dia_mes(self) -> String {
        format!("{:02}/{:02}", self.dia, self.mes)
    }
}

impl fmt::Display for Date {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:04}-{:02}-{:02}", self.ano, self.mes, self.dia)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn parse_date_aceita_so_aaaa_mm_dd_de_um_dia_que_existe() {
        assert!(Date::parse("2026-12-01").is_some());
        assert!(Date::parse("2026-12-31").is_some());
        assert!(Date::parse("2024-02-29").is_some(), "bissexto");
        assert!(Date::parse("2000-02-29").is_some(), "divisível por 400");
        for ruim in [
            "2026-12-1",
            "2026-1-01",
            "26-12-01",
            "2026/12/01",
            "01/12/2026",
            "2026-13-01",
            "2026-00-10",
            "2026-12-00",
            "2026-12-32",
            "2026-04-31",
            "2026-02-30",
            "2025-02-29",
            "1900-02-29",
            "2026-12-01T10:00",
            "2026-12-01 ",
            " 2026-12-01",
            "2026-12-01x",
            "amanhã",
            "",
            "２０２６-12-01",
        ] {
            assert!(Date::parse(ruim).is_none(), "{ruim:?} não é data");
        }
    }

    #[test]
    fn parse_date_ordena_como_as_datas_ordenam() {
        assert!(Date::parse("2026-12-05") < Date::parse("2026-12-10"));
        assert!(Date::parse("2026-12-31") < Date::parse("2027-01-01"));
        assert!(Date::parse("2026-02-28") < Date::parse("2026-03-01"));
        assert_eq!(Date::parse("2026-12-01"), Date::parse("2026-12-01"));
    }

    #[test]
    fn dias_civis_e_diferenca() {
        let a = Date::parse("1970-01-01").unwrap();
        assert_eq!(a.dias_civis(), 0);
        assert_eq!(
            Date::parse("2026-11-08")
                .unwrap()
                .dias_ate(Date::parse("2026-11-10").unwrap()),
            2
        );
        assert_eq!(
            Date::parse("2026-11-08")
                .unwrap()
                .dias_ate(Date::parse("2026-11-01").unwrap()),
            -7
        );
        assert_eq!(
            Date::parse("2026-12-31")
                .unwrap()
                .dias_ate(Date::parse("2027-01-01").unwrap()),
            1
        );
        assert_eq!(
            Date::parse("2024-02-28")
                .unwrap()
                .dias_ate(Date::parse("2024-03-01").unwrap()),
            2
        );
    }

    #[test]
    fn weekday_em_ingles() {
        assert_eq!(Date::parse("2026-11-08").unwrap().weekday(), "Sunday");
        assert_eq!(Date::parse("1970-01-01").unwrap().weekday(), "Thursday");
        assert_eq!(Date::parse("2026-01-02").unwrap().weekday(), "Friday");
    }

    #[test]
    fn formatos_do_painel_e_da_tui() {
        let d = Date::parse("2026-08-03").unwrap();
        assert_eq!(d.to_string(), "2026-08-03");
        assert_eq!(d.curto(), "03/08/26");
        assert_eq!(d.dia_mes(), "03/08");
    }
}
