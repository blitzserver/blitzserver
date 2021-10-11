use std::io::BufRead;

#[repr(i32)]
#[derive(Debug, FromPrimitive)]
pub enum TurnStatus {
    #[num_enum(default)]
    Unsubmitted = 0,
    Unfinished = 1,
    Finished = 2,
}

#[derive(Debug)]
pub struct StatusDumpNation {
    id: i32,
    turn_status: TurnStatus,
}
#[derive(Debug)]
pub struct StatusDump {
    pub turn: i32,
    pub nations: Vec<StatusDumpNation>,
}

impl StatusDump {
    pub fn from_file(file: std::fs::File) -> StatusDump {
        let mut line_iter = std::io::BufReader::new(file).lines();

        let header_regex =
            regex::Regex::new(r#"turn ([0-9]+), era [0-9]+, mods [0-9]+, turnlimit [0-9]+"#)
                .unwrap();
        let nation_regex =
            regex::Regex::new(r#"Nation	([0-9]+)	[0-9]+	[0-9]+	[0-9]	([0-9]+)	[^	]+.*"#).unwrap();
        let mut turn = 0;

        let mut nations: Vec<StatusDumpNation> = vec![];
        while let Some(Ok(line)) = line_iter.next() {
            if let Some(captures) = header_regex.captures(&line) {
                turn = captures.get(1).unwrap().as_str().parse().unwrap();
            } else if let Some(captures) = nation_regex.captures(&line) {
                nations.push(StatusDumpNation {
                    id: captures.get(1).unwrap().as_str().parse().unwrap(),
                    turn_status: TurnStatus::from(
                        captures.get(2).unwrap().as_str().parse::<i32>().unwrap(),
                    ),
                });
            }
        }
        return StatusDump { turn, nations };
    }
}
