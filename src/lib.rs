pub mod planet;
use planet::*;

#[cfg(test)]
mod tests {
    use crate::CrabRaveConstructor;
    use common_game::protocols::messages::{
        ExplorerToPlanet, OrchestratorToPlanet, PlanetToExplorer, PlanetToOrchestrator,
    };
    use crossbeam_channel::{Receiver, Sender, unbounded};

    #[test]

    fn t01_planet_initialization() -> Result<(), String> {
        println!("+++++ Test planet initialization +++++");
        let (planet_sender, _orch_receiver): (
            Sender<PlanetToOrchestrator>,
            Receiver<PlanetToOrchestrator>,
        ) = unbounded();
        let (_orch_sender, planet_receiver): (
            Sender<OrchestratorToPlanet>,
            Receiver<OrchestratorToPlanet>,
        ) = unbounded();

        let planet_to_orchestrator_channels = (planet_receiver, planet_sender);

        //planet-explorer and explorer-planet
        let (_planet_sender, _explorer_receiver): (
            Sender<PlanetToExplorer>,
            Receiver<PlanetToExplorer>,
        ) = unbounded();
        let (_explorer_sender, planet_receiver): (
            Sender<ExplorerToPlanet>,
            Receiver<ExplorerToPlanet>,
        ) = unbounded();

        let planet_to_explorer_channels = planet_receiver;

        //Construct crab-rave planet
        let crab_rave_planet = CrabRaveConstructor::new(
            0,
            planet_to_orchestrator_channels,
            planet_to_explorer_channels,
        );

        match crab_rave_planet {
            Ok(_) => Ok(()),
            Err(e) => {
                panic!("planet initialization failed. error: {}", e)
            }
        }
    }
}
