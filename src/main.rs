use std::time::SystemTime;
use common_game::components::resource::{BasicResource, BasicResourceType, Combinator, ComplexResource, ComplexResourceRequest, ComplexResourceType, Generator};
use common_game::components::rocket::Rocket;
use common_game::protocols::messages::{
    ExplorerToPlanet, OrchestratorToPlanet, PlanetToExplorer, PlanetToOrchestrator,
};
use common_game::components::planet::{PlanetAI, PlanetState};

fn main() {
    println!("Hello, world!");
}

const N_CELLS: usize = 5; // da cambiare in base al pianeta che scegliamo

pub struct MyPlanet;

impl PlanetAI for MyPlanet {

    fn handle_orchestrator_msg(&mut self, state: &mut PlanetState, generator: &Generator, combinator: &Combinator, msg: OrchestratorToPlanet) -> Option<PlanetToOrchestrator> {
        match msg {
            OrchestratorToPlanet::InternalStateRequest(..) => {
                Some(PlanetToOrchestrator::InternalStateResponse {
                    planet_id: state.id().clone(),
                    planet_state: *state, //TODO non so come passare state, vuole un PlanetState, ma PlanetState non implementa Clone o Copy, e non mi permette di fare la "move" perché arriva come shared mutable reference
                    timestamp: SystemTime::now(),
                })
            }
            OrchestratorToPlanet::Sunray( sunray) => {
                for i in 0..N_CELLS { // non ho trovato un modo per ottenere il vettore, l'unico modo penso sia quello di ciclare
                    if !state.cell(i).is_charged() {
                        state.cell_mut(i).charge(sunray);
                        return Some(PlanetToOrchestrator::SunrayAck {
                            planet_id: state.id(),
                            timestamp: SystemTime::now(),
                        });
                    }
                }
                None
            }
            _ => None
        }
    }

    fn handle_explorer_msg(&mut self, state: &mut PlanetState, generator: &Generator, combinator: &Combinator, msg: ExplorerToPlanet) -> Option<PlanetToExplorer> {
        match msg {
            ExplorerToPlanet::InternalStateRequest { explorer_id: _ } => {
                Some(PlanetToExplorer::InternalStateResponse {
                    planet_state: *state, //TODO stesso problema di sopra
                })
            }
            ExplorerToPlanet::AvailableEnergyCellRequest { explorer_id: _ } => {
                // restituisce la prima cell carica, se c'è
                for i in 0..N_CELLS {
                    if state.cell(i).is_charged() {
                        return Some(PlanetToExplorer::AvailableEnergyCellResponse {
                            available_cells: i as u32,
                        });
                    }
                }
                None
            }
            ExplorerToPlanet::SupportedResourceRequest { explorer_id: _ } => {
                Some(PlanetToExplorer::SupportedResourceResponse {
                    resource_list: None, //TODO da aggiungere la lista di risorse disponibili in base al tipo di pianeta
                })
                // TODO se non c'è niente restituisco direttamente None o come fatto appena sopra?
            }
            ExplorerToPlanet::SupportedCombinationRequest { explorer_id: _ } => {
                Some(PlanetToExplorer::SupportedCombinationResponse {
                    combination_list: None, //TODO come per SupportedResourceResponse
                })
            }
            ExplorerToPlanet::GenerateResourceRequest { explorer_id, resource } => {
                let requested_resource = resource; //TODO come accedo a msg.resource dato che è privato
                // controllo se c'è una cella carica
                let cell_idx = (0..N_CELLS).find(|&i| state.cell(i).is_charged());
                if let Some(cell_idx) = cell_idx{ // se c'è una cella carica
                    // ottengo la cella da passare al generator
                    let cell = state.cell_mut(cell_idx);
                    // pattern matching per generare la risorsa corretta
                    let generated_resource = match requested_resource {
                        BasicResourceType::Carbon => generator.make_carbon(cell).map(BasicResource::Carbon), // make_ controlla già se la risorsa è presente in generator
                        BasicResourceType::Silicon => generator.make_silicon(cell).map(BasicResource::Silicon),
                        BasicResourceType::Oxygen => generator.make_oxygen(cell).map(BasicResource::Oxygen),
                        BasicResourceType::Hydrogen => generator.make_hydrogen(cell).map(BasicResource::Hydrogen),
                    };
                    // verifico il risultato di state.generator.make...
                    match generated_resource {
                        Ok(resource) => {
                            return Some(PlanetToExplorer::GenerateResourceResponse {
                                resource: Some(resource),
                            });
                        }
                        Err(err) => {
                            println!("{}", err);
                        }
                    }
                }
                if cell_idx.is_none() {
                    println!("No available cell found"); // non dovrebbe accadere, si spera che l'explorer chieda se ce ne è una libera
                }
                Some(PlanetToExplorer::GenerateResourceResponse { //TODO ritorno come ho fatto o direttamente None?
                    resource: None
                })
            }
            ExplorerToPlanet::CombineResourceRequest { explorer_id, msg } => {
                // controllo se c'è una cella carica
                let cell_idx = (0..N_CELLS).find(|&i| state.cell(i).is_charged());
                if let Some(cell_idx) = cell_idx { // se c'è una cella carica e la risorsa è supportata vado avanti
                    // ottengo la cella da passare al generator
                    let cell = state.cell_mut(cell_idx);
                    // pattern matching per generare la risorsa corretta
                    let complex_resource = match msg {
                        ComplexResourceRequest::Water(r1, r2) => { combinator.make_water(r1, r2, cell).map(ComplexResource::Water).map_err(|(e, _, _)| {e}) }, // il map_err serve per trasformare l'err in una stringa sola in modo da poter usare il match
                        ComplexResourceRequest::Diamond(r1, r2) => { combinator.make_diamond(r1, r2, cell).map(ComplexResource::Diamond).map_err(|(e, _, _)| {e}) },
                        ComplexResourceRequest::Life(r1, r2) => { combinator.make_life(r1, r2, cell).map(ComplexResource::Life).map_err(|(e, _, _)| {e}) },
                        ComplexResourceRequest::Robot(r1, r2) => { combinator.make_robot(r1, r2, cell).map(ComplexResource::Robot).map_err(|(e, _, _)| {e}) },
                        ComplexResourceRequest::Dolphin(r1, r2) => { combinator.make_dolphin(r1, r2, cell).map(ComplexResource::Dolphin).map_err(|(e, _, _)| {e}) },
                        ComplexResourceRequest::AIPartner(r1, r2) => { combinator.make_aipartner(r1, r2, cell).map(ComplexResource::AIPartner).map_err(|(e, _, _)| {e}) },
                    };
                    // controllo il risultato di complex_resource
                    match complex_resource {
                        Ok(resource) => {
                            return Some(PlanetToExplorer::CombineResourceResponse {
                                complex_response: Some(resource),
                            });
                        }
                        Err(err) => {
                            println!("{}", err);
                        }
                    }
                }
                Some(PlanetToExplorer::CombineResourceResponse {
                    complex_response: None
                })
            }
        }
    }

    fn handle_asteroid(&mut self, state: &mut PlanetState, generator: &Generator, combinator: &Combinator) -> Option<Rocket> {
        state.take_rocket()
    }

    fn start(&mut self, state: &PlanetState) {
        println!("Planet {} AI started", state.id());
        // TODO non ho capito bene cosa deve fare planet.ai.start, deve creare il thread o lo fa l'orchestrator?
    }

    fn stop(&mut self, state: &PlanetState) {
        println!("Planet AI stopped");
        // TODO stessa cosa di "start"
    }
}
