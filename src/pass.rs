/*!

  Abstraction for running passes on netlists.

*/

use safety_net::{Instantiable, Netlist};
use std::rc::Rc;
use thiserror::Error;

use crate::netlist::PrimitiveCell;

/// Errors for running passes
#[derive(Error, Debug)]
pub enum Error {
    /// An netlist error in running the pass.
    #[error("Pass error: {0}")]
    PassError(#[from] safety_net::Error),
    /// An I/O error in writing the pass output.
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),
}

/// A pass on a netlist.
pub trait Pass {
    /// The type of Instantiable in the netlist
    type I: Instantiable;

    /// Run the pass on the given netlist and return any info as a string.
    fn run(&self, netlist: &Rc<Netlist<Self::I>>) -> Result<String, Error>;

    /// Run the pass with verification before.
    fn run_verified(&self, netlist: &Rc<Netlist<Self::I>>) -> Result<String, Error> {
        netlist.verify()?;
        self.run(netlist)
    }
}

/// Register all these passes in an enum for clap args
#[macro_export]
macro_rules! register_passes {
    ($i:ty ; $($pass:ident),+ $(,)?) => {
        /// Enum containing all registered passes for argument parsing.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
        pub enum Passes {
            $($pass),+
        }

        impl std::fmt::Display for Passes {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{:?}", self)
            }
        }

        impl Passes {
            /// Returns a boxed instance of the pass corresponding to this variant.
            pub fn get_pass(&self) -> Box<dyn eqmap::pass::Pass<I = $i>> {
                match self {
                    $(Passes::$pass => Box::new($pass),)+
                }
            }
        }
    };
}

/// A dummy pass that emits the Verilog of the netlist.
pub struct PrintVerilog;

impl Pass for PrintVerilog {
    type I = PrimitiveCell;

    fn run(&self, netlist: &Rc<Netlist<Self::I>>) -> Result<String, Error> {
        Ok(netlist.to_string())
    }
}
