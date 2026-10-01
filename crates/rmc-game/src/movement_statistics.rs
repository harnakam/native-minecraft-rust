//! EntityPlayer.addMovementStat distance selection and Java rounding.
use crate::player::Vec3;
#[derive(Clone, Copy, Debug, Default)]
pub struct MovementStatisticsContext {
    pub submerged: bool,
    pub in_water: bool,
    pub ladder: bool,
    pub on_ground: bool,
    pub sprinting: bool,
    pub sneaking: bool,
    pub riding: bool,
}

pub fn movement_increments(
    delta: Vec3,
    context: MovementStatisticsContext,
) -> Vec<(&'static str, i32)> {
    let mut result = Vec::new();
    if context.riding {
        return result;
    }
    // MathHelper.sqrt_double narrows to float before multiplication and Math.round(float).
    let horizontal = ((delta.x * delta.x + delta.z * delta.z).sqrt() as f32 * 100.0).round() as i32;
    let (id, amount) = if context.submerged {
        (
            "stat.diveOneCm",
            ((delta.x * delta.x + delta.y * delta.y + delta.z * delta.z).sqrt() as f32 * 100.0)
                .round() as i32,
        )
    } else if context.in_water {
        ("stat.swimOneCm", horizontal)
    } else if context.ladder {
        if delta.y <= 0.0 {
            return result;
        }
        // Math.round(double) returns long, then Java narrows to int.
        result.push((
            "stat.climbOneCm",
            (delta.y * 100.0 + 0.5).floor() as i64 as i32,
        ));
        return result;
    } else if context.on_ground {
        if horizontal > 0 {
            result.push(("stat.walkOneCm", horizontal));
            if context.sprinting {
                result.push(("stat.sprintOneCm", horizontal));
            } else if context.sneaking {
                result.push(("stat.crouchOneCm", horizontal));
            }
        }
        return result;
    } else {
        if horizontal <= 25 {
            return result;
        }
        ("stat.flyOneCm", horizontal)
    };
    if amount > 0 {
        result.push((id, amount));
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn branch_priority_rounding_and_airborne_threshold() {
        let delta = Vec3::new(0.3, 0.4, 0.0);
        let all = MovementStatisticsContext {
            submerged: true,
            in_water: true,
            ladder: true,
            on_ground: true,
            sprinting: true,
            sneaking: true,
            riding: false,
        };
        assert_eq!(movement_increments(delta, all), [("stat.diveOneCm", 50)]);
        assert_eq!(
            movement_increments(
                delta,
                MovementStatisticsContext {
                    submerged: false,
                    ..all
                }
            ),
            [("stat.swimOneCm", 30)]
        );
        assert_eq!(
            movement_increments(
                delta,
                MovementStatisticsContext {
                    submerged: false,
                    in_water: false,
                    ..all
                }
            ),
            [("stat.climbOneCm", 40)]
        );
        assert_eq!(
            movement_increments(
                delta,
                MovementStatisticsContext {
                    on_ground: true,
                    sprinting: true,
                    sneaking: true,
                    ..Default::default()
                }
            ),
            [("stat.walkOneCm", 30), ("stat.sprintOneCm", 30)]
        );
        assert_eq!(
            movement_increments(
                delta,
                MovementStatisticsContext {
                    on_ground: true,
                    sneaking: true,
                    ..Default::default()
                }
            ),
            [("stat.walkOneCm", 30), ("stat.crouchOneCm", 30)]
        );
        assert!(movement_increments(Vec3::new(0.25, 2.0, 0.0), Default::default()).is_empty());
        assert_eq!(
            movement_increments(Vec3::new(0.26, 2.0, 0.0), Default::default()),
            [("stat.flyOneCm", 26)]
        );
        assert!(movement_increments(
            delta,
            MovementStatisticsContext {
                riding: true,
                ..all
            }
        )
        .is_empty());
    }
    #[test]
    #[ignore = "requires local MCP919 movement statistics Java oracle"]
    fn local_java_movement_statistics_match() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tmp/movement-statistics-java-oracle.log");
        let oracle = std::fs::read_to_string(path).unwrap();
        let mut cases = 0;
        for line in oracle.lines().filter(|line| line.starts_with("MOVE|")) {
            let fields: Vec<_> = line.split('|').collect();
            let flags: u8 = fields[1].parse().unwrap();
            let context = MovementStatisticsContext {
                submerged: flags & 1 != 0,
                in_water: flags & 2 != 0,
                ladder: flags & 4 != 0,
                on_ground: flags & 8 != 0,
                sprinting: flags & 16 != 0,
                sneaking: flags & 32 != 0,
                riding: false,
            };
            let delta = Vec3::new(
                fields[2].parse().unwrap(),
                fields[3].parse().unwrap(),
                fields[4].parse().unwrap(),
            );
            let actual: std::collections::BTreeMap<_, _> =
                movement_increments(delta, context).into_iter().collect();
            let expected: std::collections::BTreeMap<_, _> = fields[5]
                .split(',')
                .filter(|s| !s.is_empty())
                .map(|s| {
                    let (id, value) = s.split_once(':').unwrap();
                    (id, value.parse::<i32>().unwrap())
                })
                .collect();
            assert_eq!(actual, expected, "{line}");
            cases += 1;
        }
        assert_eq!(cases, 6400);
        println!("{cases} MCP919 movement statistic cases match");
    }
}
