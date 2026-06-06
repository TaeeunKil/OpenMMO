use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use onlinerpg_shared::housing::{HouseData, RoomType};
use onlinerpg_terrain::{
    coords,
    trees::{filter_tree_v1_bytes_in_rects, TreeExclusionRect},
};

#[derive(Debug)]
pub struct PruneOptions {
    pub terrain: PathBuf,
    pub housing: PathBuf,
    pub margin: f32,
    pub dry_run: bool,
    pub list_houses: bool,
}

#[derive(Default)]
struct PruneStats {
    houses_read: usize,
    rects: usize,
    tiles_checked: usize,
    trees_removed: usize,
    changed_tiles: BTreeSet<(i32, i32)>,
}

fn collect_house_files(dir: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    if !dir.exists() {
        return Ok(());
    }

    for entry in fs::read_dir(dir).with_context(|| format!("read {}", dir.display()))? {
        let entry = entry.with_context(|| format!("read entry in {}", dir.display()))?;
        let path = entry.path();
        if path.is_dir() {
            collect_house_files(&path, out)?;
        } else if path.extension().is_some_and(|ext| ext == "json") {
            out.push(path);
        }
    }

    Ok(())
}

fn house_tree_rects(house: &HouseData, margin: f32) -> Vec<TreeExclusionRect> {
    house
        .rooms
        .iter()
        .filter(|room| room.floor_level == 0 && room.room_type != RoomType::Stairwell)
        .map(|room| {
            let min_x = house.origin.x + room.local_x as f32 - margin;
            let min_z = house.origin.z + room.local_z as f32 - margin;
            let max_x = house.origin.x + room.local_x as f32 + room.size_x as f32 + margin;
            let max_z = house.origin.z + room.local_z as f32 + room.size_z as f32 + margin;
            [min_x, min_z, max_x, max_z]
        })
        .collect()
}

fn print_house_list(houses: &[HouseData], margin: f32) {
    println!("Houses:");
    for house in houses {
        let rects = house_tree_rects(house, margin);
        println!(
            "  {} origin=({:.1}, {:.3}, {:.1}) rooms={} groundFootprints={}",
            house.id,
            house.origin.x,
            house.origin.y,
            house.origin.z,
            house.rooms.len(),
            rects.len()
        );
        for (idx, rect) in rects.iter().enumerate() {
            println!(
                "    rect#{idx}: x[{:.1}, {:.1}] z[{:.1}, {:.1}]",
                rect[0], rect[2], rect[1], rect[3]
            );
        }
    }
}

fn rect_tiles([min_x, min_z, max_x, max_z]: TreeExclusionRect) -> Vec<(i32, i32)> {
    let tile_min_x = coords::world_to_tile(min_x);
    let tile_max_x = coords::world_to_tile(max_x);
    let tile_min_z = coords::world_to_tile(min_z);
    let tile_max_z = coords::world_to_tile(max_z);

    let mut tiles = Vec::new();
    for tile_z in tile_min_z..=tile_max_z {
        for tile_x in tile_min_x..=tile_max_x {
            tiles.push((tile_x, tile_z));
        }
    }
    tiles
}

fn union_tiles(rects: &[TreeExclusionRect]) -> BTreeSet<(i32, i32)> {
    let mut tiles = BTreeSet::new();
    for &rect in rects {
        tiles.extend(rect_tiles(rect));
    }
    tiles
}

fn load_houses(housing_dir: &Path) -> Result<Vec<HouseData>> {
    let mut files = Vec::new();
    collect_house_files(housing_dir, &mut files)?;
    files.sort();

    let mut houses = Vec::new();
    for path in files {
        let json = fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
        let house: HouseData =
            serde_json::from_str(&json).with_context(|| format!("parse {}", path.display()))?;
        houses.push(house);
    }

    Ok(houses)
}

pub fn run(options: PruneOptions) -> Result<()> {
    let houses = load_houses(&options.housing)?;

    if options.list_houses {
        print_house_list(&houses, options.margin);
    }

    let mut rects = Vec::new();
    for house in &houses {
        rects.extend(house_tree_rects(house, options.margin));
    }

    let tiles = union_tiles(&rects);
    let mut stats = PruneStats {
        houses_read: houses.len(),
        rects: rects.len(),
        tiles_checked: tiles.len(),
        ..PruneStats::default()
    };

    for (tile_x, tile_z) in tiles {
        let path = coords::tree_path(&options.terrain, tile_x, tile_z);
        let data = match fs::read(&path) {
            Ok(data) => data,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => continue,
            Err(err) => return Err(err).with_context(|| format!("read {}", path.display())),
        };

        let Some((filtered, removed)) =
            filter_tree_v1_bytes_in_rects(tile_x, tile_z, &data, &rects)
                .with_context(|| format!("filter {}", path.display()))?
        else {
            continue;
        };

        if !options.dry_run {
            fs::write(&path, filtered).with_context(|| format!("write {}", path.display()))?;
        }

        stats.trees_removed += removed;
        stats.changed_tiles.insert((tile_x, tile_z));
    }

    println!(
        "{} house tree prune: {} house(s), {} footprint rect(s), {} tile(s) checked, {} tile(s) changed, {} tree(s) removed",
        if options.dry_run { "Dry-run" } else { "Applied" },
        stats.houses_read,
        stats.rects,
        stats.tiles_checked,
        stats.changed_tiles.len(),
        stats.trees_removed
    );

    if !stats.changed_tiles.is_empty() {
        let preview: Vec<String> = stats
            .changed_tiles
            .iter()
            .take(20)
            .map(|(x, z)| format!("({x},{z})"))
            .collect();
        let suffix = if stats.changed_tiles.len() > preview.len() {
            format!(" ... +{} more", stats.changed_tiles.len() - preview.len())
        } else {
            String::new()
        };
        println!("Changed tiles: {}{}", preview.join(", "), suffix);
    }

    Ok(())
}
