use super::diverging_scale::DivergingScale;
use crate::color::hcl::Hcl;

pub struct DivergingScheme;

impl DivergingScheme {
    pub fn rd_bu() -> DivergingScale {
        DivergingScale::new(
            vec![
                Hcl::new(0.0, 75.0, 35.0),
                Hcl::new(2.0, 68.0, 42.0),
                Hcl::new(5.0, 60.0, 50.0),
                Hcl::new(10.0, 50.0, 60.0),
                Hcl::new(15.0, 38.0, 72.0),
                Hcl::new(20.0, 25.0, 85.0),
            ],
            Hcl::new(0.0, 0.0, 97.0),
            vec![
                Hcl::new(220.0, 25.0, 85.0),
                Hcl::new(225.0, 38.0, 72.0),
                Hcl::new(230.0, 50.0, 60.0),
                Hcl::new(235.0, 60.0, 50.0),
                Hcl::new(240.0, 68.0, 42.0),
                Hcl::new(245.0, 75.0, 35.0),
            ],
            "RdBu",
        )
    }

    pub fn rd_yl_bu() -> DivergingScale {
        DivergingScale::new(
            vec![
                Hcl::new(0.0, 75.0, 35.0),
                Hcl::new(5.0, 65.0, 45.0),
                Hcl::new(15.0, 55.0, 55.0),
                Hcl::new(30.0, 45.0, 65.0),
                Hcl::new(45.0, 35.0, 75.0),
                Hcl::new(55.0, 20.0, 88.0),
            ],
            Hcl::new(50.0, 0.0, 97.0),
            vec![
                Hcl::new(210.0, 20.0, 88.0),
                Hcl::new(220.0, 35.0, 75.0),
                Hcl::new(235.0, 45.0, 65.0),
                Hcl::new(250.0, 55.0, 55.0),
                Hcl::new(265.0, 65.0, 45.0),
                Hcl::new(280.0, 75.0, 35.0),
            ],
            "RdYlBu",
        )
    }

    pub fn rd_yl_gn() -> DivergingScale {
        DivergingScale::new(
            vec![
                Hcl::new(0.0, 75.0, 35.0),
                Hcl::new(5.0, 65.0, 45.0),
                Hcl::new(25.0, 55.0, 55.0),
                Hcl::new(50.0, 45.0, 65.0),
                Hcl::new(90.0, 35.0, 75.0),
                Hcl::new(120.0, 20.0, 88.0),
            ],
            Hcl::new(80.0, 0.0, 97.0),
            vec![
                Hcl::new(140.0, 20.0, 88.0),
                Hcl::new(150.0, 35.0, 75.0),
                Hcl::new(160.0, 45.0, 65.0),
                Hcl::new(170.0, 55.0, 55.0),
                Hcl::new(180.0, 65.0, 45.0),
                Hcl::new(190.0, 75.0, 35.0),
            ],
            "RdYlGn",
        )
    }

    pub fn pi_yg() -> DivergingScale {
        DivergingScale::new(
            vec![
                Hcl::new(330.0, 70.0, 40.0),
                Hcl::new(335.0, 60.0, 50.0),
                Hcl::new(340.0, 50.0, 60.0),
                Hcl::new(345.0, 38.0, 72.0),
                Hcl::new(350.0, 25.0, 85.0),
            ],
            Hcl::new(0.0, 0.0, 97.0),
            vec![
                Hcl::new(150.0, 25.0, 85.0),
                Hcl::new(145.0, 38.0, 72.0),
                Hcl::new(140.0, 50.0, 60.0),
                Hcl::new(135.0, 60.0, 50.0),
                Hcl::new(130.0, 70.0, 40.0),
            ],
            "PiYG",
        )
    }

    pub fn br_bg() -> DivergingScale {
        DivergingScale::new(
            vec![
                Hcl::new(25.0, 70.0, 40.0),
                Hcl::new(35.0, 60.0, 50.0),
                Hcl::new(50.0, 48.0, 62.0),
                Hcl::new(75.0, 35.0, 75.0),
                Hcl::new(120.0, 20.0, 88.0),
            ],
            Hcl::new(90.0, 0.0, 97.0),
            vec![
                Hcl::new(200.0, 20.0, 88.0),
                Hcl::new(210.0, 35.0, 75.0),
                Hcl::new(220.0, 48.0, 62.0),
                Hcl::new(235.0, 60.0, 50.0),
                Hcl::new(250.0, 70.0, 40.0),
            ],
            "BrBG",
        )
    }

    pub fn pu_or() -> DivergingScale {
        DivergingScale::new(
            vec![
                Hcl::new(280.0, 70.0, 40.0),
                Hcl::new(285.0, 60.0, 50.0),
                Hcl::new(290.0, 50.0, 62.0),
                Hcl::new(295.0, 38.0, 75.0),
                Hcl::new(300.0, 25.0, 88.0),
            ],
            Hcl::new(45.0, 0.0, 97.0),
            vec![
                Hcl::new(30.0, 25.0, 88.0),
                Hcl::new(25.0, 38.0, 75.0),
                Hcl::new(20.0, 50.0, 62.0),
                Hcl::new(15.0, 60.0, 50.0),
                Hcl::new(10.0, 70.0, 40.0),
            ],
            "PuOr",
        )
    }

    pub fn spectral() -> DivergingScale {
        DivergingScale::new(
            vec![
                Hcl::new(0.0, 75.0, 35.0),
                Hcl::new(20.0, 65.0, 45.0),
                Hcl::new(45.0, 55.0, 55.0),
                Hcl::new(70.0, 45.0, 65.0),
                Hcl::new(100.0, 40.0, 75.0),
                Hcl::new(130.0, 30.0, 85.0),
            ],
            Hcl::new(80.0, 0.0, 97.0),
            vec![
                Hcl::new(180.0, 30.0, 85.0),
                Hcl::new(210.0, 40.0, 75.0),
                Hcl::new(240.0, 45.0, 65.0),
                Hcl::new(270.0, 55.0, 55.0),
                Hcl::new(300.0, 65.0, 45.0),
                Hcl::new(330.0, 75.0, 35.0),
            ],
            "Spectral",
        )
    }

    pub fn get(name: &str) -> Option<DivergingScale> {
        match name {
            "RdBu" => Some(Self::rd_bu()),
            "RdYlBu" => Some(Self::rd_yl_bu()),
            "RdYlGn" => Some(Self::rd_yl_gn()),
            "PiYG" => Some(Self::pi_yg()),
            "BrBG" => Some(Self::br_bg()),
            "PuOr" => Some(Self::pu_or()),
            "Spectral" => Some(Self::spectral()),
            _ => None,
        }
    }
}
