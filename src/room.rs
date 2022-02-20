use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, Clone)]
pub enum Room {
    MainMenu,
    IntroCutscene,

    BikiniBottom,
    SpongebobHouse,
    SquidwardHouse,
    PatrickHouse,
    ShadyShoals,
    PoliceStation,
    Treedome,
    KrustyKrab,
    ChumBucket,
    Theater,

    Poseidome,
    IndustrialPark,

    JellyfishRock,
    JellyfishCaves,
    JellyfishLake,
    JellyfishMountain,

    DowntownStreets,
    DowntownRooftops,
    DowntownLighthouse,
    DowntownSeaNeedle,

    GooLagoonBeach,
    GooLagoonCaves,
    GooLagoonPier,

    MermalairEntranceArea,
    MermalairMainChamber,
    MermalairSecurityTunnel,
    MermalairBallroom,
    MermalairVillianContainment,

    RockBottomDowntown,
    RockBottomMuseum,
    RockBottomTrench,

    SandMountainHub,
    SandMountainSlide1,
    SandMountainSlide2,
    SandMountainSlide3,

    KelpForest,
    KelpSwamps,
    KelpCaves,
    KelpVines,

    GraveyardLake,
    GraveyardShipwreck,
    GraveyardShip,
    GraveyardBoss,

    SpongebobsDream,
    SandysDream,
    SquidwardsDream,
    KrabsDream,
    PatricksDream,

    ChumBucketLab,
    ChumBucketBrain,

    SpongeballArena,
}

impl Room {
    const fn get_name(&self) -> &'static str {
        todo!()
    }
}

impl TryFrom<[u8; 4]> for Room {
    type Error = &'static str;

    fn try_from(scene_id: [u8; 4]) -> Result<Self, Self::Error> {
        match &scene_id {
            b"MNU3" => Ok(Room::MainMenu),
            b"HB00" => Ok(Room::IntroCutscene),
            b"HB01" => Ok(Room::BikiniBottom),
            b"HB02" => Ok(Room::SpongebobHouse),
            b"HB03" => Ok(Room::SquidwardHouse),
            b"HB04" => Ok(Room::PatrickHouse),
            b"HB06" => Ok(Room::ShadyShoals),
            b"HB09" => Ok(Room::PoliceStation),
            b"HB05" => Ok(Room::Treedome),
            b"HB07" => Ok(Room::KrustyKrab),
            b"HB08" => Ok(Room::ChumBucket),
            b"HB10" => Ok(Room::Theater),
            b"B101" => Ok(Room::Poseidome),
            b"B201" => Ok(Room::IndustrialPark),
            b"JF01" => Ok(Room::JellyfishRock),
            b"JF02" => Ok(Room::JellyfishCaves),
            b"JF03" => Ok(Room::JellyfishLake),
            b"JF04" => Ok(Room::JellyfishMountain),
            b"BB01" => Ok(Room::DowntownStreets),
            b"BB02" => Ok(Room::DowntownRooftops),
            b"BB03" => Ok(Room::DowntownLighthouse),
            b"BB04" => Ok(Room::DowntownSeaNeedle),
            b"GL01" => Ok(Room::GooLagoonBeach),
            b"GL02" => Ok(Room::GooLagoonCaves),
            b"GL03" => Ok(Room::GooLagoonPier),
            b"BC01" => Ok(Room::MermalairEntranceArea),
            b"BC02" => Ok(Room::MermalairMainChamber),
            b"BC03" => Ok(Room::MermalairSecurityTunnel),
            b"BC04" => Ok(Room::MermalairBallroom),
            b"BC05" => Ok(Room::MermalairVillianContainment),
            b"RB01" => Ok(Room::RockBottomDowntown),
            b"RB02" => Ok(Room::RockBottomMuseum),
            b"RB03" => Ok(Room::RockBottomTrench),
            b"SM01" => Ok(Room::SandMountainHub),
            b"SM02" => Ok(Room::SandMountainSlide1),
            b"SM03" => Ok(Room::SandMountainSlide2),
            b"SM04" => Ok(Room::SandMountainSlide3),
            b"KF01" => Ok(Room::KelpForest),
            b"KF02" => Ok(Room::KelpSwamps),
            b"KF04" => Ok(Room::KelpCaves),
            b"KF05" => Ok(Room::KelpVines),
            b"GY01" => Ok(Room::GraveyardLake),
            b"GY02" => Ok(Room::GraveyardShipwreck),
            b"GY03" => Ok(Room::GraveyardShip),
            b"GY04" => Ok(Room::GraveyardBoss),
            b"DB01" => Ok(Room::SpongebobsDream),
            b"DB02" => Ok(Room::SandysDream),
            b"DB03" => Ok(Room::SquidwardsDream),
            b"DB04" => Ok(Room::KrabsDream),
            b"DB06" => Ok(Room::PatricksDream),
            b"B302" => Ok(Room::ChumBucketLab),
            b"B303" => Ok(Room::ChumBucketBrain),
            b"PG12" => Ok(Room::SpongeballArena),
            _ => Err("Byte array did not correspond to a level."),
        }
    }
}

impl From<Room> for [u8; 4] {
    fn from(room: Room) -> [u8; 4] {
        *match room {
            Room::MainMenu => b"MNU3",
            Room::IntroCutscene => b"HB00",
            Room::BikiniBottom => b"HB01",
            Room::SpongebobHouse => b"HB02",
            Room::SquidwardHouse => b"HB03",
            Room::PatrickHouse => b"HB04",
            Room::ShadyShoals => b"HB06",
            Room::PoliceStation => b"HB09",
            Room::Treedome => b"HB05",
            Room::KrustyKrab => b"HB07",
            Room::ChumBucket => b"HB08",
            Room::Theater => b"HB10",
            Room::Poseidome => b"B101",
            Room::IndustrialPark => b"B201",
            Room::JellyfishRock => b"JF01",
            Room::JellyfishCaves => b"JF02",
            Room::JellyfishLake => b"JF03",
            Room::JellyfishMountain => b"JF04",
            Room::DowntownStreets => b"BB01",
            Room::DowntownRooftops => b"BB02",
            Room::DowntownLighthouse => b"BB03",
            Room::DowntownSeaNeedle => b"BB04",
            Room::GooLagoonBeach => b"GL01",
            Room::GooLagoonCaves => b"GL02",
            Room::GooLagoonPier => b"GL03",
            Room::MermalairEntranceArea => b"BC01",
            Room::MermalairMainChamber => b"BC02",
            Room::MermalairSecurityTunnel => b"BC03",
            Room::MermalairBallroom => b"BC04",
            Room::MermalairVillianContainment => b"BC05",
            Room::RockBottomDowntown => b"RB01",
            Room::RockBottomMuseum => b"RB02",
            Room::RockBottomTrench => b"RB03",
            Room::SandMountainHub => b"SM01",
            Room::SandMountainSlide1 => b"SM02",
            Room::SandMountainSlide2 => b"SM03",
            Room::SandMountainSlide3 => b"SM04",
            Room::KelpForest => b"KF01",
            Room::KelpSwamps => b"KF02",
            Room::KelpCaves => b"KF04",
            Room::KelpVines => b"KF05",
            Room::GraveyardLake => b"GY01",
            Room::GraveyardShipwreck => b"GY02",
            Room::GraveyardShip => b"GY03",
            Room::GraveyardBoss => b"GY04",
            Room::SpongebobsDream => b"DB01",
            Room::SandysDream => b"DB02",
            Room::SquidwardsDream => b"DB03",
            Room::KrabsDream => b"DB04",
            Room::PatricksDream => b"DB06",
            Room::ChumBucketLab => b"B302",
            Room::ChumBucketBrain => b"B303",
            Room::SpongeballArena => b"PG12",
        }
    }
}
