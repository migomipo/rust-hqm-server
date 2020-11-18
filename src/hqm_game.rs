use nalgebra::{Point3, Matrix3, Vector3, Vector2};
use std::fmt::{Display, Formatter};
use std::fmt;
use crate::hqm_parse;
use crate::hqm_parse::{HQMSkaterPacket, HQMPuckPacket};
use std::rc::Rc;
use crate::hqm_server::HQMServerConfiguration;

pub(crate) struct HQMGameWorld {
    pub(crate) objects: Vec<HQMGameObject>,
    pub(crate) rink: HQMRink,
    pub(crate) gravity: f32,
    pub(crate) limit_jump_speed: bool,
}

impl HQMGameWorld {
    pub(crate) fn create_player_object (& mut self, start: Point3<f32>, rot: Matrix3<f32>, hand: HQMSkaterHand, connected_player_index: usize) -> Option<usize> {
        let object_slot = self.find_empty_object_slot();
        if let Some(i) = object_slot {
            self.objects[i] = HQMGameObject::Player(HQMSkater::new(i, start, rot, hand, connected_player_index));
        }
        return object_slot;
    }

    pub(crate) fn create_puck_object (& mut self, start: Point3<f32>, rot: Matrix3<f32>) -> Option<usize> {
        let object_slot = self.find_empty_object_slot();
        if let Some(i) = object_slot {
            self.objects[i] = HQMGameObject::Puck(HQMPuck::new(i, start, rot));
        }
        return object_slot;
    }

    fn find_empty_object_slot(& self) -> Option<usize> {
        return self.objects.iter().position(|x| {match x {
            HQMGameObject::None  => true,
            _ => false
        }});
    }
}

pub(crate) struct HQMGame {

    pub(crate) state: HQMGameState,
    pub(crate) rules_state: HQMRulesState,
    pub(crate) world: HQMGameWorld,
    pub(crate) global_messages: Vec<Rc<HQMMessage>>,
    pub(crate) red_score: u32,
    pub(crate) blue_score: u32,
    pub(crate) period: u32,
    pub(crate) time: u32,
    pub(crate) timeout: u32,
    pub(crate) intermission: u32,
    pub(crate) paused: bool,
    pub(crate) game_id: u32,
    pub(crate) game_step: u32,
    pub(crate) game_over: bool,
    pub(crate) packet: u32,

    pub(crate) active: bool,
}

impl HQMGame {
    pub(crate) fn new (game_id: u32, config: &HQMServerConfiguration) -> Self {
        let mut object_vec = Vec::with_capacity(32);
        for _ in 0..32 {
            object_vec.push(HQMGameObject::None);
        }

        HQMGame {
            state:HQMGameState::Warmup,
            rules_state:HQMRulesState::None,
            world: HQMGameWorld {
                objects: object_vec,
                rink: HQMRink::new(30.0, 61.0, 8.5),
                gravity: 0.000680,
                limit_jump_speed: config.limit_jump_speed
            },
            global_messages: vec![],
            red_score: 0,
            blue_score: 0,
            period: 0,
            time: 30000,
            timeout: 0,
            intermission: 0,
            paused: false,

            game_over: false,
            game_id,
            game_step: 0,
            packet: 0,
            active: false,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct HQMRinkNet {
    pub(crate) team: HQMTeam,
    pub(crate) posts: Vec<(Point3<f32>, Point3<f32>, f32)>,
    pub(crate) surfaces: Vec<(Point3<f32>, Point3<f32>, Point3<f32>, Point3<f32>)>,
    pub(crate) left_post: Point3<f32>,
    pub(crate) right_post: Point3<f32>,
    pub(crate) normal: Vector3<f32>,
    pub(crate) left_post_inside: Vector3<f32>,
    pub(crate) right_post_inside: Vector3<f32>
}

impl HQMRinkNet {
    fn new(team: HQMTeam, rink_width: f32, rink_length: f32) -> Self {
        let mid_x = rink_width / 2.0;

        let (pos, rot) = match team {
            HQMTeam::Blue => (Point3::new (mid_x, 0.0, 3.5), Matrix3::identity()),
            HQMTeam::Red => (Point3::new (mid_x, 0.0, rink_length - 3.5), Matrix3::from_columns (& [-Vector3::x(), Vector3::y(), -Vector3::z()])),
            _ => panic!()
        };
        let (front_upper_left, front_upper_right, front_lower_left, front_lower_right,
            back_upper_left, back_upper_right, back_lower_left, back_lower_right) =
            (
                &pos + &rot * Vector3::new(-1.5, 1.0, 0.5),
                &pos + &rot * Vector3::new(1.5, 1.0, 0.5),
                &pos + &rot * Vector3::new(-1.5, 0.0, 0.5),
                &pos + &rot * Vector3::new(1.5, 0.0, 0.5),
                &pos + &rot * Vector3::new(-1.25, 1.0, -0.25),
                &pos + &rot * Vector3::new(1.25, 1.0, -0.25),
                &pos + &rot * Vector3::new(-1.25, 0.0, -0.5),
                &pos + &rot * Vector3::new(1.25, 0.0, -0.5)
            );

        HQMRinkNet {
            team,
            posts: vec![
                (front_lower_right.clone(), front_upper_right.clone(), 0.1875),
                (front_lower_left.clone(), front_upper_left.clone(), 0.1875),
                (front_upper_right.clone(), front_upper_left.clone(), 0.125),

                (front_lower_left.clone(), back_lower_left.clone(), 0.125),
                (front_lower_right.clone(), back_lower_right.clone(), 0.125),
                (front_upper_left.clone(), back_upper_left.clone(), 0.125),
                (back_upper_right.clone(), front_upper_right.clone(), 0.125),

                (back_lower_left.clone(), back_upper_left.clone(), 0.125),
                (back_lower_right.clone(), back_upper_right.clone(), 0.125),
                (back_lower_left.clone(), back_lower_right.clone(), 0.125),
                (back_upper_left.clone(), back_upper_right.clone(), 0.125),

            ],
            surfaces: vec![
                (back_upper_left.clone(), back_upper_right.clone(),
                 back_lower_right.clone(), back_lower_left.clone()),
                (front_upper_left.clone(), back_upper_left.clone(),
                 back_lower_left.clone(), front_lower_left.clone()),
                (front_upper_right, front_lower_right.clone(),
                 back_lower_right.clone(), back_upper_right.clone()),
                (front_upper_left.clone(), front_upper_right.clone(),
                 back_upper_right.clone(), back_upper_left.clone())
            ],
            left_post: front_lower_left.clone(),
            right_post: front_lower_right.clone(),
            normal: rot * Vector3::z(),
            left_post_inside: rot * Vector3::x(),
            right_post_inside: rot * -Vector3::x()
        }

    }
}

#[derive(Debug, Clone)]
pub(crate) struct HQMRink {
    pub(crate) planes: Vec<(Point3<f32>, Vector3<f32>)>,
    pub(crate) corners: Vec<(Point3<f32>, Vector3<f32>, f32)>,
    pub(crate) nets: Vec<HQMRinkNet>,
    pub(crate) width:f32,
    pub(crate) length:f32
}

impl HQMRink {
    fn new(width: f32, length: f32, corner_radius: f32) -> Self {

        let zero = Point3::new(0.0,0.0,0.0);
        let planes = vec![
            (zero.clone(), Vector3::y()),
            (Point3::new(0.0, 0.0, length), -Vector3::z()),
            (zero.clone(), Vector3::z()),
            (Point3::new(width, 0.0, 0.0), -Vector3::x()),
            (zero.clone(), Vector3::x()),
        ];
        let r = corner_radius;
        let wr = width - corner_radius;
        let lr = length - corner_radius;
        let corners = vec![
            (Point3::new(r, 0.0, r),   Vector3::new(-1.0, 0.0, -1.0), corner_radius),
            (Point3::new(wr, 0.0, r),  Vector3::new( 1.0, 0.0, -1.0), corner_radius),
            (Point3::new(wr, 0.0, lr), Vector3::new( 1.0, 0.0,  1.0), corner_radius),
            (Point3::new(r, 0.0, lr),  Vector3::new(-1.0, 0.0,  1.0), corner_radius)
        ];
        let red_net = HQMRinkNet::new(HQMTeam::Red, width, length);
        let blue_net = HQMRinkNet::new(HQMTeam::Blue, width, length);
        HQMRink {
            planes,
            corners,
            nets: vec![red_net, blue_net],
            width,
            length
        }
    }
}



#[derive(Debug, Clone)]
pub(crate) struct HQMBody {
    pub(crate) pos: Point3<f32>,                // Measured in meters
    pub(crate) linear_velocity: Vector3<f32>,   // Measured in meters per hundred of a second
    pub(crate) rot: Matrix3<f32>,               // Rotation matrix
    pub(crate) angular_velocity: Vector3<f32>,  // Measured in radians per hundred of a second
    pub(crate) rot_mul: Vector3<f32>
}

#[derive(Debug, Clone)]
pub(crate) struct HQMSkater {
    pub(crate) index: usize,
    pub(crate) connected_player_index: usize,
    pub(crate) body: HQMBody,
    pub(crate) stick_pos: Point3<f32>,        // Measured in meters
    pub(crate) stick_velocity: Vector3<f32>,  // Measured in meters per hundred of a second
    pub(crate) stick_rot: Matrix3<f32>,       // Rotation matrix
    pub(crate) head_rot: f32,                 // Radians
    pub(crate) body_rot: f32,                 // Radians
    pub(crate) height: f32,
    pub(crate) input: HQMPlayerInput,
    pub(crate) jumped_last_frame: bool,
    pub(crate) stick_placement: Vector2<f32>,      // Azimuth and inclination in radians
    pub(crate) stick_placement_delta: Vector2<f32>, // Change in azimuth and inclination per hundred of a second
    pub(crate) collision_balls: Vec<HQMSkaterCollisionBall>,
    pub(crate) hand: HQMSkaterHand
}

impl HQMSkater {

    fn get_collision_balls(pos: &Point3<f32>, rot: &Matrix3<f32>, linear_velocity: &Vector3<f32>) -> Vec<HQMSkaterCollisionBall> {
        let mut collision_balls = Vec::with_capacity(6);
        collision_balls.push(HQMSkaterCollisionBall::from_skater(Vector3::new(0.0, 0.0, 0.0), pos, rot, linear_velocity, 0.225));
        collision_balls.push(HQMSkaterCollisionBall::from_skater(Vector3::new(0.25, 0.3125, 0.0), pos, rot, linear_velocity, 0.25));
        collision_balls.push(HQMSkaterCollisionBall::from_skater(Vector3::new(-0.25, 0.3125, 0.0), pos, rot, linear_velocity, 0.25));
        collision_balls.push(HQMSkaterCollisionBall::from_skater(Vector3::new(-0.1875, -0.1875, 0.0), pos, rot, linear_velocity, 0.1875));
        collision_balls.push(HQMSkaterCollisionBall::from_skater(Vector3::new(0.1875, -0.1875, 0.0), pos, rot, linear_velocity, 0.1875));
        collision_balls.push(HQMSkaterCollisionBall::from_skater(Vector3::new(0.0, 0.5, 0.0), pos, & rot, linear_velocity, 0.1875));
        collision_balls
    }

    fn new(object_index: usize, pos: Point3<f32>, rot: Matrix3<f32>, hand: HQMSkaterHand, connected_player_index: usize) -> Self {
        let linear_velocity = Vector3::new (0.0, 0.0, 0.0);
        let collision_balls = HQMSkater::get_collision_balls(&pos, &rot, &linear_velocity);
        HQMSkater {
            index:object_index,
            connected_player_index,
            body: HQMBody {
                pos: pos.clone(),
                linear_velocity,
                rot,
                angular_velocity: Vector3::new (0.0, 0.0, 0.0),
                rot_mul: Vector3::new (2.75, 6.16, 2.35)
            },
            stick_pos: pos.clone(),
            stick_velocity: Vector3::new (0.0, 0.0, 0.0),
            stick_rot: Matrix3::identity(),
            head_rot: 0.0,
            body_rot: 0.0,
            height: 0.75,
            input: HQMPlayerInput::default(),
            jumped_last_frame: false,
            stick_placement: Vector2::new(0.0, 0.0),
            stick_placement_delta: Vector2::new(0.0, 0.0),
            hand,
            collision_balls
        }
    }

    pub(crate) fn get_packet(&self) -> HQMSkaterPacket {
        let rot = hqm_parse::convert_matrix(31, & self.body.rot);
        let stick_rot = hqm_parse::convert_matrix(25, & self.stick_rot);

        HQMSkaterPacket {
            pos: (get_position (17, 1024.0 * self.body.pos.x),
                  get_position (17, 1024.0 * self.body.pos.y),
                  get_position (17, 1024.0 * self.body.pos.z)),
            rot,
            stick_pos: (get_position (13, 1024.0 * (self.stick_pos.x - self.body.pos.x + 4.0)),
                        get_position (13, 1024.0 * (self.stick_pos.y - self.body.pos.y + 4.0)),
                        get_position (13, 1024.0 * (self.stick_pos.z - self.body.pos.z + 4.0))),
            stick_rot,
            head_rot: get_position (16, (self.head_rot + 2.0) * 8192.0),
            body_rot: get_position (16, (self.body_rot + 2.0) * 8192.0)
        }
    }

}

#[derive(Debug, Clone)]
pub(crate) struct HQMSkaterCollisionBall {
    pub(crate) offset: Vector3<f32>,
    pub(crate) pos: Point3<f32>,
    pub(crate) velocity: Vector3<f32>,
    pub(crate) radius: f32

}

impl HQMSkaterCollisionBall {
    fn from_skater(offset: Vector3<f32>, skater_pos: & Point3<f32>, skater_rot: & Matrix3<f32>, velocity: & Vector3<f32>, radius: f32) -> Self {
        let pos = skater_pos + skater_rot * offset;
        HQMSkaterCollisionBall {
            offset,
            pos,
            velocity: velocity.clone_owned(),
            radius
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct HQMPlayerInput {
    pub(crate) stick_angle: f32,
    pub(crate) turn: f32,
    pub(crate) unknown: f32,
    pub(crate) fwbw: f32,
    pub(crate) stick: Vector2<f32>,
    pub(crate) head_rot: f32,
    pub(crate) body_rot: f32,
    pub(crate) keys: u32,
}

impl Default for HQMPlayerInput {
    fn default() -> Self {
        HQMPlayerInput {
            stick_angle: 0.0,
            turn: 0.0,
            unknown: 0.0,
            fwbw: 0.0,
            stick: Vector2::new(0.0, 0.0),
            head_rot: 0.0,
            body_rot: 0.0,
            keys: 0
        }
    }
}

impl HQMPlayerInput {
    pub fn jump (&self) -> bool { self.keys & 0x1 != 0}
    pub fn crouch (&self) -> bool { self.keys & 0x2 != 0}
    pub fn join_red (&self) -> bool { self.keys & 0x4 != 0}
    pub fn join_blue (&self) -> bool { self.keys & 0x8 != 0}
    pub fn shift (&self) -> bool { self.keys & 0x10 != 0}
    pub fn spectate (&self) -> bool { self.keys & 0x20 != 0}
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub(crate) enum HQMSkaterHand {
    Left, Right
}

#[derive(Debug, Clone)]
pub(crate) struct HQMPuck {
    pub(crate) index: usize,
    pub(crate) body: HQMBody,
    pub(crate) radius: f32,
    pub(crate) height: f32,
    pub(crate) last_player_index: [Option<usize>; 4],
}

impl HQMPuck {
    fn new(object_index:usize,pos: Point3<f32>, rot: Matrix3<f32>) -> Self {
        HQMPuck {
            index:object_index,
            body: HQMBody {
                pos,
                linear_velocity: Vector3::new(0.0, 0.0, 0.0),
                rot,
                angular_velocity: Vector3::new(0.0,0.0,0.0),
                rot_mul: Vector3::new(223.5, 128.0, 223.5)
            },
            radius: 0.125,
            height: 0.0412500016391,
            last_player_index: [None; 4]
        }
    }

    pub(crate) fn get_packet(&self) -> HQMPuckPacket {
        let rot = hqm_parse::convert_matrix(31, & self.body.rot);
        HQMPuckPacket {
            pos: (get_position (17, 1024.0 * self.body.pos.x),
                  get_position (17, 1024.0 * self.body.pos.y),
                  get_position (17, 1024.0 * self.body.pos.z)),
            rot
        }
    }

}

#[derive(Debug, Clone)]
pub(crate) enum HQMGameObject {
    None,
    Player(HQMSkater),
    Puck(HQMPuck),
}



#[derive(Debug, Clone)]
pub(crate) enum HQMMessage {
    PlayerUpdate {
        player_name: Vec<u8>,
        team: HQMTeam,
        player_index: usize,
        object_index: Option<usize>,
        in_server: bool,
    },
    Goal {
        team: HQMTeam,
        goal_player_index: Option<usize>,
        assist_player_index: Option<usize>,
    },
    Chat {
        player_index: Option<usize>,
        message: Vec<u8>,
    },
}

pub(crate) struct HQMFaceoffPosition {
    pub(crate) abbreviation: String,
    pub(crate) faceoff_offsets: Vec<Vector3<f32>> // To store multiple faceoff positions as needed
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum HQMTeam {
    Spec,
    Red,
    Blue,
}

impl HQMTeam {
    pub(crate) fn get_num(self) -> u32 {
        match self {
            HQMTeam::Red => 0,
            HQMTeam::Blue => 1,
            HQMTeam::Spec => u32::MAX
        }
    }
}

impl Display for HQMTeam {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            HQMTeam::Red => write!(f, "Red"),
            HQMTeam::Blue => write!(f, "Blue"),
            HQMTeam::Spec => write!(f, "Spec")
        }
    }
}


#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum HQMGameState {
    Warmup,
    Game,
    Intermission,
    Timeout,
    Paused,
    GameOver,
}

impl Display for HQMGameState {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            HQMGameState::Warmup => write!(f, "Warmup"),
            HQMGameState::Game => write!(f, "Game"),
            HQMGameState::Intermission => write!(f, "Intermission"),
            HQMGameState::Timeout => write!(f, "Timeout"),
            HQMGameState::Paused => write!(f, "Paused"),
            HQMGameState::GameOver => write!(f, "Game Over"),

        }
    }
}





#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum HQMRulesState {
    None,
    OffsideWarning,
    IcingWarning,
    DualWarning,
    Offside,
    Icing,
}

impl HQMRulesState {
    pub(crate) fn update_num(self) -> u32 {
        match self {
            HQMRulesState::None => 0,
            HQMRulesState::OffsideWarning => 1,
            HQMRulesState::IcingWarning => 2,
            HQMRulesState::DualWarning => 3,
            HQMRulesState::Offside => 4,
            HQMRulesState::Icing => 8,

        }
    }
}

impl Display for HQMRulesState {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            HQMRulesState::None => write!(f, "None"),
            HQMRulesState::OffsideWarning => write!(f, "Offside Warning"),
            HQMRulesState::IcingWarning => write!(f, "Icing Warning"),
            HQMRulesState::DualWarning => write!(f, "Offside Warning + Icing Warning"),
            HQMRulesState::Offside => write!(f, "Offside"),
            HQMRulesState::Icing => write!(f, "Icing"),

        }
    }
}

fn get_position (bits: u32, v: f32) -> u32 {
    let temp = v as i32;
    if temp < 0 {
        0
    } else if temp > ((1 << bits) - 1) {
        ((1 << bits) - 1) as u32
    } else {
        temp as u32
    }
}