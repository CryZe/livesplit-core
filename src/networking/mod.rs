//! The networking module provides functionality to communicate with various
//! speedrunning related websites, such as Splits.io to upload and download runs
//! and Speedrun.com to query and submit to the leaderboards of most games. The
//! module is optional and is not compiled in by default.

pub mod speedrun_com;
pub mod splits_io;
