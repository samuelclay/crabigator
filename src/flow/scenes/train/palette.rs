use super::*;
#[derive(Clone, Copy)]
#[allow(non_snake_case)]
pub(super) struct Pal {
    pub skyTop: u32,
    pub skyHz: u32,
    pub cloud: u32,
    pub cloudShade: u32,
    pub hillFar: u32,
    pub hill: u32,
    pub grass: u32,
    pub grass2: u32,
    pub wheat: u32,
    pub wheat2: u32,
    pub plough: u32,
    pub plough2: u32,
    pub rape: u32,
    pub sheep: u32,
    pub hedge: u32,
    pub tree: u32,
    pub treeLit: u32,
    pub treeShade: u32,
    pub treeFar: u32,
    pub treeFarLit: u32,
    pub floor: u32,
    pub wall: u32,
    pub roof: u32,
    pub roofDark: u32,
    pub slate: u32,
    pub houseWin: u32,
    pub water: u32,
    pub waterHi: u32,
    pub reed: u32,
    pub ballast: u32,
    pub ballast2: u32,
    pub ballastFar: u32,
    pub verge: u32,
    pub sleeper: u32,
    pub rail: u32,
    pub railFar: u32,
    pub shadow: u32,
    pub road: u32,
    pub platform: u32,
    pub platformFace: u32,
    pub platformEdge: u32,
    pub canopy: u32,
    pub valance: u32,
    pub brick: u32,
    pub steel: u32,
    pub pole: u32,
    pub post: u32,
    pub cream: u32,
    pub crimson: u32,
    pub loco: u32,
    pub locoVent: u32,
    pub yellow: u32,
    pub carRoof: u32,
    pub vent: u32,
    pub grille: u32,
    pub glass: u32,
    pub sideGlass: u32,
    pub bogie: u32,
    pub lamp: u32,
    pub beam: u32,
    pub spill: u32,
    pub soot: u32,
    pub haze: u32,
    pub bird: u32,
    pub rain: u32,
}
pub(super) const DAY: Pal = Pal {
    skyTop: 0x3f88d6,
    skyHz: 0xb6dff5,
    cloud: 0xf6f9fc,
    cloudShade: 0xd0dfec,
    hillFar: 0x93b8c8,
    hill: 0x6f9f94,
    grass: 0x6fae4c,
    grass2: 0x86bd58,
    wheat: 0xd8be5e,
    wheat2: 0xc4a84c,
    plough: 0x8c6a4a,
    plough2: 0x76583c,
    rape: 0xdcc94e,
    sheep: 0xf2f0e6,
    hedge: 0x3b7432,
    tree: 0x2f6e36,
    treeLit: 0x4b8d42,
    treeShade: 0x1f4a26,
    treeFar: 0x386c52,
    treeFarLit: 0x4b7f63,
    floor: 0x2a4a26,
    wall: 0xe4d8c2,
    roof: 0xb04e3c,
    roofDark: 0x7e3428,
    slate: 0x5d636c,
    houseWin: 0x3a4552,
    water: 0x4f93c8,
    waterHi: 0xa6d6f0,
    reed: 0x6a7a3a,
    ballast: 0x8b8073,
    ballast2: 0x9a8f80,
    ballastFar: 0x9b9184,
    verge: 0x79a052,
    sleeper: 0x5c4838,
    rail: 0xc6cad0,
    railFar: 0xb0b4ba,
    shadow: 0x2e2a26,
    road: 0x606368,
    platform: 0xbcb7ad,
    platformFace: 0x8c877e,
    platformEdge: 0xe8c640,
    canopy: 0x4e535a,
    valance: 0xd9d2c2,
    brick: 0x9c5638,
    steel: 0x434a54,
    pole: 0x5a4636,
    post: 0x34363c,
    cream: 0xebdfba,
    crimson: 0xa8322d,
    loco: 0x1f4f8f,
    locoVent: 0x163a6a,
    yellow: 0xf2c230,
    carRoof: 0x8e9399,
    vent: 0x6e737a,
    grille: 0x3c3c40,
    glass: 0x29323c,
    sideGlass: 0xb9ae90,
    bogie: 0x1e1f22,
    lamp: 0xfff4d0,
    beam: 0xfff0b8,
    spill: 0xffd27a,
    soot: 0x1b1b1d,
    haze: 0x9aa0a8,
    bird: 0x2c3440,
    rain: 0xc4d4ea,
};
pub(super) const NIGHT: Pal = Pal {
    skyTop: NIGHT_ZENITH,
    skyHz: NIGHT_HORIZON,
    cloud: 0x2a3348,
    cloudShade: 0x1a2132,
    hillFar: 0x131c2e,
    hill: 0x0d1522,
    grass: 0x0f1a13,
    grass2: 0x121e15,
    wheat: 0x1d1c13,
    wheat2: 0x181710,
    plough: 0x15110d,
    plough2: 0x110e0b,
    rape: 0x1f1e11,
    sheep: 0x3a3a36,
    hedge: 0x070d09,
    tree: 0x08100b,
    treeLit: 0x0e1912,
    treeShade: 0x030604,
    treeFar: 0x091411,
    treeFarLit: 0x0d1b18,
    floor: 0x060b07,
    wall: 0x24252b,
    roof: 0x241517,
    roofDark: 0x180e10,
    slate: 0x15171d,
    houseWin: 0xffc35a,
    water: 0x0b1828,
    waterHi: 0x4a6a8e,
    reed: 0x0d130a,
    ballast: 0x1c1b1a,
    ballast2: 0x222120,
    ballastFar: 0x191817,
    verge: 0x0e170f,
    sleeper: 0x0f0c0a,
    rail: 0x68717e,
    railFar: 0x4c535e,
    shadow: 0x070707,
    road: 0x18191c,
    platform: 0x3a3934,
    platformFace: 0x22211e,
    platformEdge: 0x7a6a2a,
    canopy: 0x121418,
    valance: 0x4a4740,
    brick: 0x28160e,
    steel: 0x171a1f,
    pole: 0x110e0b,
    post: 0x131417,
    cream: 0x4a4536,
    crimson: 0x3c1512,
    loco: 0x0c1a2e,
    locoVent: 0x08121f,
    yellow: 0x6a5620,
    carRoof: 0x2a2d33,
    vent: 0x1e2025,
    grille: 0x111214,
    glass: 0xffd47e,
    sideGlass: 0xffd47e,
    bogie: 0x09090a,
    lamp: 0xffffff,
    beam: 0xfff0c0,
    spill: 0xffc860,
    soot: 0x5c5d63,
    haze: 0x30333a,
    bird: 0x10141c,
    rain: 0x6a7c9c,
};
pub(super) fn palette(night: f64, smoke: f64, storm: f64) -> Pal {
    let kn = q(night, 8.);
    let ks = q(smoke, 6.);
    let kb = q(storm, 6.);
    let mut p = Pal {
        skyTop: mix(DAY.skyTop, NIGHT.skyTop, kn),
        skyHz: mix(DAY.skyHz, NIGHT.skyHz, kn),
        cloud: mix(DAY.cloud, NIGHT.cloud, kn),
        cloudShade: mix(DAY.cloudShade, NIGHT.cloudShade, kn),
        hillFar: mix(DAY.hillFar, NIGHT.hillFar, kn),
        hill: mix(DAY.hill, NIGHT.hill, kn),
        grass: mix(DAY.grass, NIGHT.grass, kn),
        grass2: mix(DAY.grass2, NIGHT.grass2, kn),
        wheat: mix(DAY.wheat, NIGHT.wheat, kn),
        wheat2: mix(DAY.wheat2, NIGHT.wheat2, kn),
        plough: mix(DAY.plough, NIGHT.plough, kn),
        plough2: mix(DAY.plough2, NIGHT.plough2, kn),
        rape: mix(DAY.rape, NIGHT.rape, kn),
        sheep: mix(DAY.sheep, NIGHT.sheep, kn),
        hedge: mix(DAY.hedge, NIGHT.hedge, kn),
        tree: mix(DAY.tree, NIGHT.tree, kn),
        treeLit: mix(DAY.treeLit, NIGHT.treeLit, kn),
        treeShade: mix(DAY.treeShade, NIGHT.treeShade, kn),
        treeFar: mix(DAY.treeFar, NIGHT.treeFar, kn),
        treeFarLit: mix(DAY.treeFarLit, NIGHT.treeFarLit, kn),
        floor: mix(DAY.floor, NIGHT.floor, kn),
        wall: mix(DAY.wall, NIGHT.wall, kn),
        roof: mix(DAY.roof, NIGHT.roof, kn),
        roofDark: mix(DAY.roofDark, NIGHT.roofDark, kn),
        slate: mix(DAY.slate, NIGHT.slate, kn),
        houseWin: mix(DAY.houseWin, NIGHT.houseWin, kn),
        water: mix(DAY.water, NIGHT.water, kn),
        waterHi: mix(DAY.waterHi, NIGHT.waterHi, kn),
        reed: mix(DAY.reed, NIGHT.reed, kn),
        ballast: mix(DAY.ballast, NIGHT.ballast, kn),
        ballast2: mix(DAY.ballast2, NIGHT.ballast2, kn),
        ballastFar: mix(DAY.ballastFar, NIGHT.ballastFar, kn),
        verge: mix(DAY.verge, NIGHT.verge, kn),
        sleeper: mix(DAY.sleeper, NIGHT.sleeper, kn),
        rail: mix(DAY.rail, NIGHT.rail, kn),
        railFar: mix(DAY.railFar, NIGHT.railFar, kn),
        shadow: mix(DAY.shadow, NIGHT.shadow, kn),
        road: mix(DAY.road, NIGHT.road, kn),
        platform: mix(DAY.platform, NIGHT.platform, kn),
        platformFace: mix(DAY.platformFace, NIGHT.platformFace, kn),
        platformEdge: mix(DAY.platformEdge, NIGHT.platformEdge, kn),
        canopy: mix(DAY.canopy, NIGHT.canopy, kn),
        valance: mix(DAY.valance, NIGHT.valance, kn),
        brick: mix(DAY.brick, NIGHT.brick, kn),
        steel: mix(DAY.steel, NIGHT.steel, kn),
        pole: mix(DAY.pole, NIGHT.pole, kn),
        post: mix(DAY.post, NIGHT.post, kn),
        cream: mix(DAY.cream, NIGHT.cream, kn),
        crimson: mix(DAY.crimson, NIGHT.crimson, kn),
        loco: mix(DAY.loco, NIGHT.loco, kn),
        locoVent: mix(DAY.locoVent, NIGHT.locoVent, kn),
        yellow: mix(DAY.yellow, NIGHT.yellow, kn),
        carRoof: mix(DAY.carRoof, NIGHT.carRoof, kn),
        vent: mix(DAY.vent, NIGHT.vent, kn),
        grille: mix(DAY.grille, NIGHT.grille, kn),
        glass: mix(DAY.glass, NIGHT.glass, kn),
        sideGlass: mix(DAY.sideGlass, NIGHT.sideGlass, kn),
        bogie: mix(DAY.bogie, NIGHT.bogie, kn),
        lamp: mix(DAY.lamp, NIGHT.lamp, kn),
        beam: mix(DAY.beam, NIGHT.beam, kn),
        spill: mix(DAY.spill, NIGHT.spill, kn),
        soot: mix(DAY.soot, NIGHT.soot, kn),
        haze: mix(DAY.haze, NIGHT.haze, kn),
        bird: mix(DAY.bird, NIGHT.bird, kn),
        rain: mix(DAY.rain, NIGHT.rain, kn),
    };
    if ks > 0. {
        p.skyTop = mix(p.skyTop, mix(0x6e7680, 0x1c1d21, kn), ks);
        p.skyHz = mix(p.skyHz, mix(0xadb2b8, 0x34363b, kn), ks);
        p.cloud = mix(p.cloud, mix(0xc4c8cc, 0x3a3c42, kn), ks);
        p.cloudShade = mix(p.cloudShade, mix(0x8e949a, 0x26282c, kn), ks);
        p.hillFar = grey(p.hillFar, 0.5 * ks);
        p.hill = grey(p.hill, 0.5 * ks);
        p.grass = grey(p.grass, 0.5 * ks);
        p.grass2 = grey(p.grass2, 0.5 * ks);
        p.wheat = grey(p.wheat, 0.5 * ks);
        p.wheat2 = grey(p.wheat2, 0.5 * ks);
        p.plough = grey(p.plough, 0.5 * ks);
        p.plough2 = grey(p.plough2, 0.5 * ks);
        p.rape = grey(p.rape, 0.5 * ks);
        p.sheep = grey(p.sheep, 0.5 * ks);
        p.hedge = grey(p.hedge, 0.5 * ks);
        p.tree = grey(p.tree, 0.5 * ks);
        p.treeLit = grey(p.treeLit, 0.5 * ks);
        p.treeShade = grey(p.treeShade, 0.5 * ks);
        p.treeFar = grey(p.treeFar, 0.5 * ks);
        p.treeFarLit = grey(p.treeFarLit, 0.5 * ks);
        p.floor = grey(p.floor, 0.5 * ks);
        p.wall = grey(p.wall, 0.5 * ks);
        p.roof = grey(p.roof, 0.5 * ks);
        p.roofDark = grey(p.roofDark, 0.5 * ks);
        p.slate = grey(p.slate, 0.5 * ks);
        p.water = grey(p.water, 0.5 * ks);
        p.waterHi = grey(p.waterHi, 0.5 * ks);
        p.reed = grey(p.reed, 0.5 * ks);
        p.ballast = grey(p.ballast, 0.5 * ks);
        p.ballast2 = grey(p.ballast2, 0.5 * ks);
        p.ballastFar = grey(p.ballastFar, 0.5 * ks);
        p.verge = grey(p.verge, 0.5 * ks);
        p.sleeper = grey(p.sleeper, 0.5 * ks);
        p.rail = grey(p.rail, 0.5 * ks);
        p.railFar = grey(p.railFar, 0.5 * ks);
        p.shadow = grey(p.shadow, 0.5 * ks);
        p.road = grey(p.road, 0.5 * ks);
        p.platform = grey(p.platform, 0.5 * ks);
        p.platformFace = grey(p.platformFace, 0.5 * ks);
        p.platformEdge = grey(p.platformEdge, 0.5 * ks);
        p.canopy = grey(p.canopy, 0.5 * ks);
        p.valance = grey(p.valance, 0.5 * ks);
        p.brick = grey(p.brick, 0.5 * ks);
        p.steel = grey(p.steel, 0.5 * ks);
        p.pole = grey(p.pole, 0.5 * ks);
        p.post = grey(p.post, 0.5 * ks);
    }
    if kb > 0. {
        p.skyTop = mix(p.skyTop, mix(0x1b2540, 0x5080f, kn), kb);
        p.skyHz = mix(p.skyHz, mix(0x4f6284, 0x141c30, kn), kb);
        p.cloud = mix(p.cloud, mix(0x6a7a96, 0x1e2840, kn), kb);
        p.cloudShade = mix(p.cloudShade, mix(0x4a5874, 0x121a2c, kn), kb);
        let cast = mix(0x23345a, 0x0a1226, kn);
        p.hillFar = mix(p.hillFar, cast, 0.42 * kb);
        p.hill = mix(p.hill, cast, 0.42 * kb);
        p.grass = mix(p.grass, cast, 0.42 * kb);
        p.grass2 = mix(p.grass2, cast, 0.42 * kb);
        p.wheat = mix(p.wheat, cast, 0.42 * kb);
        p.wheat2 = mix(p.wheat2, cast, 0.42 * kb);
        p.plough = mix(p.plough, cast, 0.42 * kb);
        p.plough2 = mix(p.plough2, cast, 0.42 * kb);
        p.rape = mix(p.rape, cast, 0.42 * kb);
        p.sheep = mix(p.sheep, cast, 0.42 * kb);
        p.hedge = mix(p.hedge, cast, 0.42 * kb);
        p.tree = mix(p.tree, cast, 0.42 * kb);
        p.treeLit = mix(p.treeLit, cast, 0.42 * kb);
        p.treeShade = mix(p.treeShade, cast, 0.42 * kb);
        p.treeFar = mix(p.treeFar, cast, 0.42 * kb);
        p.treeFarLit = mix(p.treeFarLit, cast, 0.42 * kb);
        p.floor = mix(p.floor, cast, 0.42 * kb);
        p.wall = mix(p.wall, cast, 0.42 * kb);
        p.roof = mix(p.roof, cast, 0.42 * kb);
        p.roofDark = mix(p.roofDark, cast, 0.42 * kb);
        p.slate = mix(p.slate, cast, 0.42 * kb);
        p.water = mix(p.water, cast, 0.42 * kb);
        p.waterHi = mix(p.waterHi, cast, 0.42 * kb);
        p.reed = mix(p.reed, cast, 0.42 * kb);
        p.ballast = mix(p.ballast, cast, 0.42 * kb);
        p.ballast2 = mix(p.ballast2, cast, 0.42 * kb);
        p.ballastFar = mix(p.ballastFar, cast, 0.42 * kb);
        p.verge = mix(p.verge, cast, 0.42 * kb);
        p.sleeper = mix(p.sleeper, cast, 0.42 * kb);
        p.rail = mix(p.rail, cast, 0.42 * kb);
        p.railFar = mix(p.railFar, cast, 0.42 * kb);
        p.shadow = mix(p.shadow, cast, 0.42 * kb);
        p.road = mix(p.road, cast, 0.42 * kb);
        p.platform = mix(p.platform, cast, 0.42 * kb);
        p.platformFace = mix(p.platformFace, cast, 0.42 * kb);
        p.platformEdge = mix(p.platformEdge, cast, 0.42 * kb);
        p.canopy = mix(p.canopy, cast, 0.42 * kb);
        p.valance = mix(p.valance, cast, 0.42 * kb);
        p.brick = mix(p.brick, cast, 0.42 * kb);
        p.steel = mix(p.steel, cast, 0.42 * kb);
        p.pole = mix(p.pole, cast, 0.42 * kb);
        p.post = mix(p.post, cast, 0.42 * kb);
        p.cream = mix(p.cream, cast, 0.18 * kb);
        p.crimson = mix(p.crimson, cast, 0.18 * kb);
        p.loco = mix(p.loco, cast, 0.18 * kb);
        p.locoVent = mix(p.locoVent, cast, 0.18 * kb);
        p.yellow = mix(p.yellow, cast, 0.18 * kb);
        p.carRoof = mix(p.carRoof, cast, 0.18 * kb);
        p.vent = mix(p.vent, cast, 0.18 * kb);
        p.grille = mix(p.grille, cast, 0.18 * kb);
        p.bogie = mix(p.bogie, cast, 0.18 * kb);
        p.glass = mix(p.glass, NIGHT.glass, 0.75 * kb);
        p.sideGlass = mix(p.sideGlass, NIGHT.sideGlass, 0.75 * kb);
        p.houseWin = mix(p.houseWin, NIGHT.houseWin, 0.6 * kb);
    }
    p
}
