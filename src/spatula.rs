use strum_macros::EnumIter;

#[derive(EnumIter, Debug, Clone, Copy, PartialEq, Eq)]
pub enum Spatula {
    // Bikini Bottom
    OnTopOfThePineapple,
    OnTopOfShadyShoals,
    OnTopOfTheChumBucket,
    SpongebobsCloset,
    AnnoySquidward,
    AmbushAtTheTreeDome,
    InfestationAtTheKrustyKrab,
    AWallJumpInTheBucket,

    // Jellyfish Fields
    TopOfTheHill,
    CowaBungee,
    Spelunking,
    PatricksDilemma,
    NavigateTheCanyonsAndMesas,
    DrainTheLake,
    SlideLeap,
    DefeatKingJellyfish,

    // Downtown Bikini Bottom
    EndOfTheRoad,
    LearnSandysMoves,
    TikisGoBoom,
    AcrossTheRooftops,
    SwinginSandy,
    AmbushInTheLighthouse,
    ExtremeBungee,
    ComeBackWithTheCruiseBubble,

    // Goo Lagoon
    KingOfTheCastle,
    ConnectTheTowers,
    SaveTheChildren,
    OverTheMoat,
    ThroughTheSeaCaves,
    CleanOutTheBumperBoats,
    SlipAndSlideUnderThePier,
    TowerBungee,

    // Poseidome
    RumbleAtThePoseidome,

    // Rock Bottom
    GetToTheMuseum,
    SlipSlidingAway,
    ReturnTheMuseumsArt,
    SwingalongSpatula,
    PlunderingRobotsInTheMuseum,
    AcrossTheTrenchOfDarkness,
    LasersAreFunAndGoodForYou,
    HowInTarnationDoYouGetThere,

    // Mermalair
    TopOfTheEntranceAreaML,
    TopOfTheComputerArea,
    ShutDownTheSecuritySystem,
    TheFunnelMachines,
    TheSpinningTowersOfPower,
    TopOfTheSecurityTunnel,
    CompleteTheRollingBallRoom,
    DefeatPrawn,

    // Sand Mountain
    FrostyBungee,
    TopOfTheLodge,
    DefeatRobotsOnGuppyMound,
    BeatMrsPuffsTime,
    DefeatRobotsOnFlounderHill,
    BeatBubbleBuddysTime,
    DefeatRobotsOnSandMountain,
    BeatLarrysTime,

    // Industrial Park
    RoboPatrickAhoy,

    // Kelp Forest
    ThroughTheWoods,
    FindAllTheLostCampers,
    TikiRoundup,
    DownInTheSwamp,
    ThroughTheKelpCaves,
    PowerCrystalCrisis,
    KelpVineSlide,
    BeatMermaidMansTime,

    // Flying Dutchman's Graveyard
    TopOfTheEntranceAreaFDG,
    APathThroughTheGoo,
    GooTankerAhoy,
    TopOfTheStackOfShips,
    ShipwreckBungee,
    DestroyTheRobotShip,
    GetAloftThereMatey,
    DefeatTheFlyingDutchman,

    // SpongeBob's Dream
    AcrossTheDreamscape,
    FollowTheBouncingBall,
    SlidingTexasStyle,
    SwingersAhoy,
    MusicIsInTheEarOfTheBeholder,
    KrabbyPattyPlatforms,
    SuperBounce,
    HereYouGo,

    // Chum Bucket Lab
    KahRahTae,
    TheSmallShallRuleOrNot,
}

impl TryFrom<(u32, u32)> for Spatula {
    type Error = &'static str;

    fn try_from(value: (u32, u32)) -> Result<Self, Self::Error> {
        use Spatula::*;
        match value {
            (0, 0) => Ok(OnTopOfThePineapple),
            (0, 1) => Ok(OnTopOfShadyShoals),
            (0, 2) => Ok(OnTopOfTheChumBucket),
            (0, 3) => Ok(SpongebobsCloset),
            (0, 4) => Ok(AnnoySquidward),
            (0, 5) => Ok(AmbushAtTheTreeDome),
            (0, 6) => Ok(InfestationAtTheKrustyKrab),
            (0, 7) => Ok(AWallJumpInTheBucket),

            // Jellyfish Fields
            (1, 0) => Ok(TopOfTheHill),
            (1, 1) => Ok(CowaBungee),
            (1, 2) => Ok(Spelunking),
            (1, 3) => Ok(PatricksDilemma),
            (1, 4) => Ok(NavigateTheCanyonsAndMesas),
            (1, 5) => Ok(DrainTheLake),
            (1, 6) => Ok(SlideLeap),
            (1, 7) => Ok(DefeatKingJellyfish),

            // Downtown Bikini Bottom
            (2, 0) => Ok(EndOfTheRoad),
            (2, 1) => Ok(LearnSandysMoves),
            (2, 2) => Ok(TikisGoBoom),
            (2, 3) => Ok(AcrossTheRooftops),
            (2, 4) => Ok(SwinginSandy),
            (2, 5) => Ok(AmbushInTheLighthouse),
            (2, 6) => Ok(ExtremeBungee),
            (2, 7) => Ok(ComeBackWithTheCruiseBubble),

            // Goo Lagoon
            (3, 0) => Ok(KingOfTheCastle),
            (3, 1) => Ok(ConnectTheTowers),
            (3, 2) => Ok(SaveTheChildren),
            (3, 3) => Ok(OverTheMoat),
            (3, 4) => Ok(ThroughTheSeaCaves),
            (3, 5) => Ok(CleanOutTheBumperBoats),
            (3, 6) => Ok(SlipAndSlideUnderThePier),
            (3, 7) => Ok(TowerBungee),

            // Poseidome
            (4, 0) => Ok(RumbleAtThePoseidome),

            // Rock Bottom
            (5, 0) => Ok(GetToTheMuseum),
            (5, 1) => Ok(SlipSlidingAway),
            (5, 2) => Ok(ReturnTheMuseumsArt),
            (5, 3) => Ok(SwingalongSpatula),
            (5, 4) => Ok(PlunderingRobotsInTheMuseum),
            (5, 5) => Ok(AcrossTheTrenchOfDarkness),
            (5, 6) => Ok(LasersAreFunAndGoodForYou),
            (5, 7) => Ok(HowInTarnationDoYouGetThere),

            // Mermalair
            (6, 0) => Ok(TopOfTheEntranceAreaML),
            (6, 1) => Ok(TopOfTheComputerArea),
            (6, 2) => Ok(ShutDownTheSecuritySystem),
            (6, 3) => Ok(TheFunnelMachines),
            (6, 4) => Ok(TheSpinningTowersOfPower),
            (6, 5) => Ok(TopOfTheSecurityTunnel),
            (6, 6) => Ok(CompleteTheRollingBallRoom),
            (6, 7) => Ok(DefeatPrawn),

            // Sand Mountain
            (7, 0) => Ok(FrostyBungee),
            (7, 1) => Ok(TopOfTheLodge),
            (7, 2) => Ok(DefeatRobotsOnGuppyMound),
            (7, 3) => Ok(BeatMrsPuffsTime),
            (7, 4) => Ok(DefeatRobotsOnFlounderHill),
            (7, 5) => Ok(BeatBubbleBuddysTime),
            (7, 6) => Ok(DefeatRobotsOnSandMountain),
            (7, 7) => Ok(BeatLarrysTime),

            // Industrial Park
            (8, 0) => Ok(RoboPatrickAhoy),

            // Kelp Forest
            (9, 0) => Ok(ThroughTheWoods),
            (9, 1) => Ok(FindAllTheLostCampers),
            (9, 2) => Ok(TikiRoundup),
            (9, 3) => Ok(DownInTheSwamp),
            (9, 4) => Ok(ThroughTheKelpCaves),
            (9, 5) => Ok(PowerCrystalCrisis),
            (9, 6) => Ok(KelpVineSlide),
            (9, 7) => Ok(BeatMermaidMansTime),

            // Flying Dutchman's Graveyard
            (10, 0) => Ok(TopOfTheEntranceAreaFDG),
            (10, 1) => Ok(APathThroughTheGoo),
            (10, 2) => Ok(GooTankerAhoy),
            (10, 3) => Ok(TopOfTheStackOfShips),
            (10, 4) => Ok(ShipwreckBungee),
            (10, 5) => Ok(DestroyTheRobotShip),
            (10, 6) => Ok(GetAloftThereMatey),
            (10, 7) => Ok(DefeatTheFlyingDutchman),

            // SpongeBob's Dream
            (11, 0) => Ok(AcrossTheDreamscape),
            (11, 1) => Ok(FollowTheBouncingBall),
            (11, 2) => Ok(SlidingTexasStyle),
            (11, 3) => Ok(SwingersAhoy),
            (11, 4) => Ok(MusicIsInTheEarOfTheBeholder),
            (11, 5) => Ok(KrabbyPattyPlatforms),
            (11, 6) => Ok(SuperBounce),
            (11, 7) => Ok(HereYouGo),

            // Chum Bucket Lab,
            (12, 0) => Ok(KahRahTae),
            (12, 1) => Ok(TheSmallShallRuleOrNot),
            _ => Err(""),
        }
    }
}

impl From<Spatula> for (u32, u32) {
    fn from(spatula: Spatula) -> Self {
        use Spatula::*;
        match spatula {
            // Bikini Bottom
            OnTopOfThePineapple => (0, 0),
            OnTopOfShadyShoals => (0, 1),
            OnTopOfTheChumBucket => (0, 2),
            SpongebobsCloset => (0, 3),
            AnnoySquidward => (0, 4),
            AmbushAtTheTreeDome => (0, 5),
            InfestationAtTheKrustyKrab => (0, 6),
            AWallJumpInTheBucket => (0, 7),

            // Jellyfish Fields
            TopOfTheHill => (1, 0),
            CowaBungee => (1, 1),
            Spelunking => (1, 2),
            PatricksDilemma => (1, 3),
            NavigateTheCanyonsAndMesas => (1, 4),
            DrainTheLake => (1, 5),
            SlideLeap => (1, 6),
            DefeatKingJellyfish => (1, 7),

            // Downtown Bikini Bottom
            EndOfTheRoad => (2, 0),
            LearnSandysMoves => (2, 1),
            TikisGoBoom => (2, 2),
            AcrossTheRooftops => (2, 3),
            SwinginSandy => (2, 4),
            AmbushInTheLighthouse => (2, 5),
            ExtremeBungee => (2, 6),
            ComeBackWithTheCruiseBubble => (2, 7),

            // Goo Lagoon
            KingOfTheCastle => (3, 0),
            ConnectTheTowers => (3, 1),
            SaveTheChildren => (3, 2),
            OverTheMoat => (3, 3),
            ThroughTheSeaCaves => (3, 4),
            CleanOutTheBumperBoats => (3, 5),
            SlipAndSlideUnderThePier => (3, 6),
            TowerBungee => (3, 7),

            // Poseidome
            RumbleAtThePoseidome => (4, 0),

            // Rock Bottom
            GetToTheMuseum => (5, 0),
            SlipSlidingAway => (5, 1),
            ReturnTheMuseumsArt => (5, 2),
            SwingalongSpatula => (5, 3),
            PlunderingRobotsInTheMuseum => (5, 4),
            AcrossTheTrenchOfDarkness => (5, 5),
            LasersAreFunAndGoodForYou => (5, 6),
            HowInTarnationDoYouGetThere => (5, 7),

            // Mermalair
            TopOfTheEntranceAreaML => (6, 0),
            TopOfTheComputerArea => (6, 1),
            ShutDownTheSecuritySystem => (6, 2),
            TheFunnelMachines => (6, 3),
            TheSpinningTowersOfPower => (6, 4),
            TopOfTheSecurityTunnel => (6, 5),
            CompleteTheRollingBallRoom => (6, 6),
            DefeatPrawn => (6, 7),

            // Sand Mountain
            FrostyBungee => (7, 0),
            TopOfTheLodge => (7, 1),
            DefeatRobotsOnGuppyMound => (7, 2),
            BeatMrsPuffsTime => (7, 3),
            DefeatRobotsOnFlounderHill => (7, 4),
            BeatBubbleBuddysTime => (7, 5),
            DefeatRobotsOnSandMountain => (7, 6),
            BeatLarrysTime => (7, 7),

            // Industrial Park
            RoboPatrickAhoy => (8, 0),

            // Kelp Forest
            ThroughTheWoods => (9, 0),
            FindAllTheLostCampers => (9, 1),
            TikiRoundup => (9, 2),
            DownInTheSwamp => (9, 3),
            ThroughTheKelpCaves => (9, 4),
            PowerCrystalCrisis => (9, 5),
            KelpVineSlide => (9, 6),
            BeatMermaidMansTime => (9, 7),

            // Flying Dutchman's Graveyard
            TopOfTheEntranceAreaFDG => (10, 0),
            APathThroughTheGoo => (10, 1),
            GooTankerAhoy => (10, 2),
            TopOfTheStackOfShips => (10, 3),
            ShipwreckBungee => (10, 4),
            DestroyTheRobotShip => (10, 5),
            GetAloftThereMatey => (10, 6),
            DefeatTheFlyingDutchman => (10, 7),

            // Spongebob's Dream
            AcrossTheDreamscape => (11, 0),
            FollowTheBouncingBall => (11, 1),
            SlidingTexasStyle => (11, 2),
            SwingersAhoy => (11, 3),
            MusicIsInTheEarOfTheBeholder => (11, 4),
            KrabbyPattyPlatforms => (11, 5),
            SuperBounce => (11, 6),
            HereYouGo => (11, 7),

            // Chum Bucket Lab
            KahRahTae => (12, 0),
            TheSmallShallRuleOrNot => (12, 1),
        }
    }
}
