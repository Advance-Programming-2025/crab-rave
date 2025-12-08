use crossbeam_channel::{Receiver, Sender};
use common_game::components::planet::{Planet, PlanetAI, PlanetState, PlanetType};
use common_game::components::resource::BasicResourceType::Carbon;
use common_game::components::resource::{
    BasicResource, BasicResourceType, Combinator, ComplexResource, ComplexResourceRequest, ComplexResourceType, Generator, GenericResource
};
use common_game::components::rocket::Rocket;
use common_game::protocols::messages::{
    ExplorerToPlanet, OrchestratorToPlanet, PlanetToExplorer, PlanetToOrchestrator,
};

use common_game::logging::EventType::{
    MessageOrchestratorToPlanet, MessagePlanetToExplorer, MessagePlanetToOrchestrator,
};
use common_game::logging::{ActorType, Channel, EventType, LogEvent, Payload};
use stacks::{
    get_charged_cell_index, get_free_cell_index, initialize_free_cell_stack,
    peek_charged_cell_index, push_charged_cell, push_free_cell,
};
///////////////////////////////////////////////////////////////////////////////////////////
// CrabRave Constructor
///////////////////////////////////////////////////////////////////////////////////////////
const RCV_MSG_LOG_CHNL: Channel = Channel::Info; // change this 2 in order to change the channel of the logs
const ACK_MSG_LOG_CHNL: Channel = Channel::Info;

#[macro_export]
macro_rules! log_msg {
    ($event:expr, $channel:expr) => {{
        match $channel {
            Channel::Info => {
                log::info!("{}", $event);
            }
            Channel::Debug => {
                log::debug!("{}", $event);
            }
            Channel::Error => {
                log::error!("{}", $event);
            }
            Channel::Trace => {
                log::trace!("{}", $event);
            }
            Channel::Warning => {
                log::warn!("{}", $event);
            }
        }
    }};
}
pub struct CrabRaveConstructor;

impl CrabRaveConstructor {
    #[allow(clippy::new_ret_no_self)]
    pub fn new(
        id: u32,
        orchestrator_channels: (Receiver<OrchestratorToPlanet>, Sender<PlanetToOrchestrator>),
        explorer_channels: Receiver<ExplorerToPlanet>,
    ) -> Result<Planet, String> {
        let (planet_type, ai, gen_rules, comb_rules, orchestrator_channels, explorer_channels) = (
            PlanetType::D,
            AI::new(),
            vec![Carbon],
            vec![],
            orchestrator_channels,
            explorer_channels,
        );
        //LOG
        let mut payload = Payload::new();
        payload.insert(
            String::from("gen_rules"),
            gen_rules.iter().map(|x| x.res_to_string() + ", ").collect(),
        );
        payload.insert("Message".to_string(), "New planet created".to_string());

        //LOG
        let new_planet = Planet::new(
            id,
            planet_type,
            Box::new(ai),
            gen_rules,
            comb_rules,
            orchestrator_channels,
            explorer_channels,
        )?;
        //LOG
        let event = LogEvent::new(
            ActorType::Orchestrator,
            0u64,
            ActorType::Planet,
            id.to_string(),
            EventType::MessageOrchestratorToPlanet,
            Channel::Info,
            payload,
        );
        log::info!("{}", event);
        //LOG
        Ok(new_planet)
    }
}

///////////////////////////////////////////////////////////////////////////////////////////
// PlanetAI
///////////////////////////////////////////////////////////////////////////////////////////

pub struct AI;

impl AI {
    fn new() -> Self {
        //LOG
        let mut payload = Payload::new();
        payload.insert(String::from("Message"), String::from("New AI created"));
        let event = LogEvent::new(
            ActorType::Planet,
            0u64,
            ActorType::Planet,
            "0".to_string(),
            EventType::InternalPlanetAction,
            Channel::Info,
            payload,
        );
        log::info!("{}", event);
        //LOG
        initialize_free_cell_stack();
        Self
    }
}

impl PlanetAI for AI {
    fn handle_orchestrator_msg(
        &mut self,
        state: &mut PlanetState,
        _generator: &Generator,
        _combinator: &Combinator,
        msg: OrchestratorToPlanet,
    ) -> Option<PlanetToOrchestrator> {
        match msg {
            OrchestratorToPlanet::InternalStateRequest => {
                //LOG
                let mut payload = Payload::new();
                payload.insert(
                    "PlanetState".to_string(),
                    format!("{:?}", PlanetState::to_dummy(state)),
                );
                payload.insert(
                    String::from("Message"),
                    String::from("Internal state request"),
                );
                let event = LogEvent::new(
                    ActorType::Orchestrator,
                    0u64,
                    ActorType::Planet,
                    state.id().to_string(),
                    EventType::MessageOrchestratorToPlanet,
                    RCV_MSG_LOG_CHNL,
                    payload,
                );
                log_msg!(event, RCV_MSG_LOG_CHNL);
                let mut payload_ris = Payload::new();
                payload_ris.insert(
                    String::from("ACK Response of InternalStateRequest"),
                    format!(
                        "planet_id: {:?}, planet_state: {:?}",
                        state.id(),
                        PlanetState::to_dummy(state)
                    ),
                );
                let event_ris = LogEvent::new(
                    ActorType::Planet,
                    state.id(),
                    ActorType::Orchestrator,
                    "0".to_string(),
                    MessagePlanetToOrchestrator,
                    ACK_MSG_LOG_CHNL,
                    payload_ris,
                );
                log_msg!(event_ris, ACK_MSG_LOG_CHNL);
                //LOG
                Some(PlanetToOrchestrator::InternalStateResponse {
                    planet_id: state.id(),
                    planet_state: PlanetState::to_dummy(state),
                })
            }
            OrchestratorToPlanet::Sunray(sunray) => {
                let mut payload_ris = Payload::new();
                let mut ris = None;
                if let Some(idx) = get_free_cell_index() {
                    state.cell_mut(idx as usize).charge(sunray);
                    push_charged_cell(idx);

                    payload_ris.insert("Message".to_string(), "SunrayAck".to_string());
                    payload_ris.insert(String::from("Result"), String::from("EnergyCell charged"));
                    payload_ris.insert(String::from("EnergyCell index"), format!("{}", idx));
                    payload_ris.insert(
                        String::from("Response data"),
                        format!("planet_id: {}", state.id()),
                    );

                    ris = Some(PlanetToOrchestrator::SunrayAck {
                        planet_id: state.id(),
                    })
                } else {
                    payload_ris.insert("Response to".to_string(), "Sunray".to_string());
                    payload_ris.insert(String::from("Result"), String::from("No free cell found"));
                }

                //LOG
                let mut payload = Payload::new();
                payload.insert(String::from("Message"), String::from("Sunray"));
                let event = LogEvent::new(
                    ActorType::Orchestrator,
                    0u64,
                    ActorType::Planet,
                    state.id().to_string(),
                    EventType::MessageOrchestratorToPlanet,
                    RCV_MSG_LOG_CHNL,
                    payload,
                );
                log_msg!(event, RCV_MSG_LOG_CHNL);
                let event_ris = LogEvent::new(
                    ActorType::Planet,
                    state.id(),
                    ActorType::Orchestrator,
                    "0".to_string(),
                    MessagePlanetToOrchestrator,
                    ACK_MSG_LOG_CHNL,
                    payload_ris,
                );
                log_msg!(event_ris, ACK_MSG_LOG_CHNL);
                //LOG
                ris
            }
            _ => {
                //LOG TODO add more information
                let mut payload = Payload::new();
                payload.insert(
                    String::from("Message"),
                    "message behaviour not defined".to_string(),
                );
                let event = LogEvent::new(
                    ActorType::Orchestrator,
                    0u64,
                    ActorType::Planet,
                    state.id().to_string(),
                    EventType::MessageOrchestratorToPlanet,
                    Channel::Error,
                    payload,
                );
                log_msg!(event, Channel::Error);
                None
            }
        }
    }

    fn handle_explorer_msg(
        &mut self,
        state: &mut PlanetState,
        generator: &Generator,
        combinator: &Combinator,
        msg: ExplorerToPlanet,
    ) -> Option<PlanetToExplorer> {
        match msg {
            ExplorerToPlanet::AvailableEnergyCellRequest { explorer_id: id } => {
                // restituisce la prima cell carica, se c'è

                let mut payload_ris = Payload::new();

                let mut n_available_cells = 0;
                for i in 0..N_CELLS {
                    if state.cell(i).is_charged() {
                        n_available_cells += 1;
                    }
                }

                let mut ris = None;
                // if let Some(idx) = peek_charged_cell_index() {
                    payload_ris.insert(
                        "Message".to_string(),
                        "AvailableEnergyCellResponse".to_string(),
                    );
                    payload_ris.insert(String::from("Result"), "EnergyCell available".to_string());
                    payload_ris.insert(String::from("EnergyCell index"), format!("{}", n_available_cells));
                    ris = Some(PlanetToExplorer::AvailableEnergyCellResponse {
                        available_cells: n_available_cells,
                    });
                // } else {
                //     payload_ris.insert(
                //         "Response to".to_string(),
                //         "AvailableEnergyCellRequest".to_string(),
                //     );
                //     payload_ris.insert(
                //         String::from("Result"),
                //         "No EnergyCell available".to_string(),
                //     );
                // }

                //LOG
                let mut payload = Payload::new();
                payload.insert(
                    String::from("Message"),
                    String::from("Available EnergyCell request"),
                );
                let event = LogEvent::new(
                    ActorType::Explorer,
                    id,
                    ActorType::Planet,
                    state.id().to_string(),
                    EventType::MessageExplorerToPlanet,
                    RCV_MSG_LOG_CHNL,
                    payload,
                );
                let event_ris = LogEvent::new(
                    ActorType::Planet,
                    state.id(),
                    ActorType::Orchestrator,
                    "0".to_string(),
                    MessagePlanetToOrchestrator,
                    ACK_MSG_LOG_CHNL,
                    payload_ris,
                );
                log_msg!(event, RCV_MSG_LOG_CHNL);
                log_msg!(event_ris, ACK_MSG_LOG_CHNL);
                //LOG
                ris
            }
            ExplorerToPlanet::SupportedResourceRequest { explorer_id: id } => {
                //LOG
                let mut payload = Payload::new();
                let mut payload_ris = Payload::new();
                payload.insert(
                    String::from("Message"),
                    String::from("Supported resource request"),
                );
                payload_ris.insert(
                    String::from("Message"),
                    "Supported resource response".to_string(),
                );
                payload_ris.insert(
                    "Result".to_string(),
                    format!("resource_list: {:?})", generator.all_available_recipes()),
                );
                let event = LogEvent::new(
                    ActorType::Explorer,
                    id,
                    ActorType::Planet,
                    state.id().to_string(),
                    EventType::MessageExplorerToPlanet,
                    RCV_MSG_LOG_CHNL,
                    payload,
                );
                let event_ris = LogEvent::new(
                    ActorType::Planet,
                    state.id(),
                    ActorType::Explorer,
                    id.to_string(),
                    EventType::MessagePlanetToExplorer,
                    ACK_MSG_LOG_CHNL,
                    payload_ris,
                );
                log_msg!(event, RCV_MSG_LOG_CHNL);
                log_msg!(event_ris, ACK_MSG_LOG_CHNL);
                //LOG
                Some(PlanetToExplorer::SupportedResourceResponse {
                    resource_list: generator.all_available_recipes(),
                })
            }
            ExplorerToPlanet::SupportedCombinationRequest { explorer_id: id } => {
                //LOG
                let mut payload = Payload::new();
                let mut payload_ris = Payload::new();
                payload.insert(
                    String::from("Message"),
                    String::from("Supported combination request"),
                );
                payload_ris.insert(
                    String::from("Message"),
                    "Supported combination response".to_string(),
                );
                payload_ris.insert(
                    "Result".to_string(),
                    format!("combination_list: {:?}", combinator.all_available_recipes()),
                );
                let event = LogEvent::new(
                    ActorType::Explorer,
                    id,
                    ActorType::Planet,
                    state.id().to_string(),
                    EventType::MessageExplorerToPlanet,
                    RCV_MSG_LOG_CHNL,
                    payload,
                );
                let event_ris = LogEvent::new(
                    ActorType::Planet,
                    state.id(),
                    ActorType::Explorer,
                    id.to_string(),
                    MessagePlanetToExplorer,
                    ACK_MSG_LOG_CHNL,
                    payload_ris,
                );
                log_msg!(event, RCV_MSG_LOG_CHNL);
                log_msg!(event_ris, ACK_MSG_LOG_CHNL);
                //LOG
                Some(PlanetToExplorer::SupportedCombinationResponse {
                    combination_list: combinator.all_available_recipes(),
                })
            }

            //TODO use explorer_id to send the gen resource to correct Explorer
            ExplorerToPlanet::GenerateResourceRequest {
                explorer_id,
                resource,
            } => {
                //LOG
                let mut payload = Payload::new();
                let mut payload_ris = Payload::new();
                payload.insert(
                    "Message".to_string(),
                    "Generate resource request".to_string(),
                );
                payload.insert("requested resource".to_string(), format!("{:?}", resource));
                let event = LogEvent::new(
                    ActorType::Explorer,
                    explorer_id,
                    ActorType::Planet,
                    state.id().to_string(),
                    EventType::MessageExplorerToPlanet,
                    RCV_MSG_LOG_CHNL,
                    payload,
                );
                log_msg!(event, RCV_MSG_LOG_CHNL);
                log::info!("{}", event);
                //LOG
                let requested_resource = resource;
                // controllo se c'è una cella carica

                if let Some(cell_idx) = get_charged_cell_index() {
                    // se c'è una cella carica
                    // ottengo la cella da passare al generator
                    //TODO add a detailed log (debug)
                    let cell = state.cell_mut(cell_idx as usize);
                    // pattern matching per generare la risorsa corretta
                    let generated_resource = match requested_resource {
                        BasicResourceType::Carbon => {
                            generator.make_carbon(cell).map(BasicResource::Carbon)
                        } // make_ controlla già se la risorsa è presente in generator
                        BasicResourceType::Silicon => {
                            generator.make_silicon(cell).map(BasicResource::Silicon)
                        }
                        BasicResourceType::Oxygen => {
                            generator.make_oxygen(cell).map(BasicResource::Oxygen)
                        }
                        BasicResourceType::Hydrogen => {
                            generator.make_hydrogen(cell).map(BasicResource::Hydrogen)
                        }
                    };
                    // verifico il risultato di state.generator.make...
                    match generated_resource {
                        Ok(resource) => {
                            payload_ris.insert(
                                "Mesage".to_string(),
                                "Generated Resource Response".to_string(),
                            );

                            payload_ris.insert(
                                "Result".to_string(),
                                format!("produced resource: {:?}", resource),
                            );

                            let event_ris = LogEvent::new(
                                ActorType::Planet,
                                state.id(),
                                ActorType::Explorer,
                                explorer_id.to_string(),
                                MessagePlanetToExplorer,
                                ACK_MSG_LOG_CHNL,
                                payload_ris,
                            );

                            log_msg!(event_ris, ACK_MSG_LOG_CHNL);

                            //LOG
                            push_free_cell(cell_idx);
                            return Some(PlanetToExplorer::GenerateResourceResponse {
                                resource: Some(resource),
                            });
                        }
                        Err(err) => {
                            push_charged_cell(cell_idx);
                            println!("{}", err); // TODO add logs to print errors
                        }
                    }
                } else {
                    //TODO change this in a error log
                    println!("No available cell found"); // non dovrebbe accadere, si spera che l'explorer chieda se ce ne è una libera
                }
                //LOG

                payload_ris.insert(
                    String::from("Response to"),
                    "Generated resource request".to_string(),
                );

                payload_ris.insert(
                    String::from("Result"),
                    format!("resource: {:?} not produced", resource),
                );

                let event_ris = LogEvent::new(
                    ActorType::Planet,
                    state.id(),
                    ActorType::Explorer,
                    explorer_id.to_string(),
                    MessagePlanetToExplorer,
                    ACK_MSG_LOG_CHNL,
                    payload_ris,
                );

                log_msg!(event_ris, ACK_MSG_LOG_CHNL);

                //LOG
                Some(PlanetToExplorer::GenerateResourceResponse { resource: None })
            }
            //TODO use explorer_id to send the gen resource to correct Explorer
            ExplorerToPlanet::CombineResourceRequest {
                explorer_id,
                msg: resource,
            } => {
                //renamed msg to resouce to be more consistent with generateresourcerequest
                // searching the index of the first free cell
                //LOG
                let mut payload = Payload::new();
                let mut payload_ris = Payload::new();
                payload.insert(
                    "Message".to_string(),
                    "Combine resource request".to_string(),
                );
                payload.insert(
                    "requested complex resource".to_string(),
                    format!("{:?}", resource),
                );
                let event = LogEvent::new(
                    ActorType::Explorer,
                    explorer_id,
                    ActorType::Planet,
                    state.id().to_string(),
                    EventType::MessageExplorerToPlanet,
                    RCV_MSG_LOG_CHNL,
                    payload,
                );

                log_msg!(event, RCV_MSG_LOG_CHNL);
                //LOG

                // TODO add log of the response
                if let Some(cell_idx) = get_charged_cell_index() {
                    let cell = state.cell_mut(cell_idx as usize);
                    let complex_resource: Result<
                        ComplexResource,
                        (String, GenericResource, GenericResource),
                    > = match resource {
                        ComplexResourceRequest::Water(r1, r2) => combinator
                            .make_water(r1, r2, cell)
                            .map(ComplexResource::Water)
                            .map_err(|(e, r1, r2)| {
                                (
                                    e,
                                    GenericResource::BasicResources(BasicResource::Hydrogen(r1)),
                                    GenericResource::BasicResources(BasicResource::Oxygen(r2)),
                                )
                            }),
                        ComplexResourceRequest::Diamond(r1, r2) => combinator
                            .make_diamond(r1, r2, cell)
                            .map(ComplexResource::Diamond)
                            .map_err(|(e, r1, r2)| {
                                (
                                    e,
                                    GenericResource::BasicResources(BasicResource::Carbon(r1)),
                                    GenericResource::BasicResources(BasicResource::Carbon(r2)),
                                )
                            }),
                        ComplexResourceRequest::Life(r1, r2) => combinator
                            .make_life(r1, r2, cell)
                            .map(ComplexResource::Life)
                            .map_err(|(e, r1, r2)| {
                                (
                                    e,
                                    GenericResource::ComplexResources(ComplexResource::Water(r1)),
                                    GenericResource::BasicResources(BasicResource::Carbon(r2)),
                                )
                            }),

                        ComplexResourceRequest::Robot(r1, r2) => combinator
                            .make_robot(r1, r2, cell)
                            .map(ComplexResource::Robot)
                            .map_err(|(e, r1, r2)| {
                                (
                                    e,
                                    GenericResource::BasicResources(BasicResource::Silicon(r1)),
                                    GenericResource::ComplexResources(ComplexResource::Life(r2)),
                                )
                            }),

                        ComplexResourceRequest::Dolphin(r1, r2) => combinator
                            .make_dolphin(r1, r2, cell)
                            .map(ComplexResource::Dolphin)
                            .map_err(|(e, r1, r2)| {
                                (
                                    e,
                                    GenericResource::ComplexResources(ComplexResource::Water(r1)),
                                    GenericResource::ComplexResources(ComplexResource::Life(r2)),
                                )
                            }),

                        ComplexResourceRequest::AIPartner(r1, r2) => combinator
                            .make_aipartner(r1, r2, cell)
                            .map(ComplexResource::AIPartner)
                            .map_err(|(e, r1, r2)| {
                                (
                                    e,
                                    GenericResource::ComplexResources(ComplexResource::Robot(r1)),
                                    GenericResource::ComplexResources(ComplexResource::Diamond(r2)),
                                )
                            }),
                    };
                    // checking the result of complex_resource
                    match complex_resource {
                        Ok(resource) => {
                            //LOG

                            payload_ris.insert(
                                "Message".to_string(),
                                "Combine resource response".to_string(),
                            );

                            payload_ris.insert(
                                "Result".to_string(),
                                format!("produced resource: {:?}", resource),
                            );

                            let event_ris = LogEvent::new(
                                ActorType::Planet,
                                state.id(),
                                ActorType::Explorer,
                                explorer_id.to_string(),
                                EventType::MessageExplorerToPlanet,
                                ACK_MSG_LOG_CHNL,
                                payload_ris,
                            );

                            //LOG

                            log_msg!(event_ris, ACK_MSG_LOG_CHNL);

                            push_free_cell(cell_idx);
                            Some(PlanetToExplorer::CombineResourceResponse {
                                complex_response: Ok(resource),
                            })
                        }
                        Err(err) => {
                            push_charged_cell(cell_idx);
                            //LOG

                            payload_ris.insert(
                                "Message".to_string(),
                                "Combine resource response".to_string(),
                            );

                            payload_ris.insert("Result".to_string(), format!("{:?}", err));

                            let event_ris = LogEvent::new(
                                ActorType::Planet,
                                state.id(),
                                ActorType::Explorer,
                                explorer_id.to_string(),
                                EventType::MessageExplorerToPlanet,
                                ACK_MSG_LOG_CHNL,
                                payload_ris,
                            );

                            log_msg!(event_ris, ACK_MSG_LOG_CHNL);

                            //LOG

                            Some(PlanetToExplorer::CombineResourceResponse {
                                complex_response: Err(err),
                            })
                        }
                    }
                } else {
                    //TODO change this to log error
                    println!("No available cell found");
                    let (ret1, ret2) = match resource {
                        ComplexResourceRequest::Water(r1, r2) => (
                            GenericResource::BasicResources(BasicResource::Hydrogen(r1)),
                            GenericResource::BasicResources(BasicResource::Oxygen(r2)),
                        ),
                        ComplexResourceRequest::AIPartner(r1, r2) => (
                            GenericResource::ComplexResources(ComplexResource::Robot(r1)),
                            GenericResource::ComplexResources(ComplexResource::Diamond(r2)),
                        ),
                        ComplexResourceRequest::Life(r1, r2) => (
                            GenericResource::ComplexResources(ComplexResource::Water(r1)),
                            GenericResource::BasicResources(BasicResource::Carbon(r2)),
                        ),
                        ComplexResourceRequest::Diamond(r1, r2) => (
                            GenericResource::BasicResources(BasicResource::Carbon(r1)),
                            GenericResource::BasicResources(BasicResource::Carbon(r2)),
                        ),
                        ComplexResourceRequest::Dolphin(r1, r2) => (
                            GenericResource::ComplexResources(ComplexResource::Water(r1)),
                            GenericResource::ComplexResources(ComplexResource::Life(r2)),
                        ),
                        ComplexResourceRequest::Robot(r1, r2) => (
                            GenericResource::BasicResources(BasicResource::Silicon(r1)),
                            GenericResource::ComplexResources(ComplexResource::Life(r2)),
                        ),
                    };

                    //LOG

                    payload_ris.insert(
                        "Message".to_string(),
                        "Combine resource response".to_string(),
                    );

                    payload_ris.insert(
                        "Result".to_string(),
                        format!("Err: no available cell. {:?}, {:?}", ret1, ret2),
                    );

                    let event_ris = LogEvent::new(
                        ActorType::Planet,
                        state.id(),
                        ActorType::Explorer,
                        explorer_id.to_string(),
                        EventType::MessageExplorerToPlanet,
                        ACK_MSG_LOG_CHNL,
                        payload_ris,
                    );

                    log_msg!(event_ris, ACK_MSG_LOG_CHNL);

                    //LOG

                    Some(PlanetToExplorer::CombineResourceResponse {
                        complex_response: Err(("no available cell".to_string(), ret1, ret2)),
                    })
                }
            }
        }
    }

    fn handle_asteroid(
        &mut self,
        state: &mut PlanetState,
        _generator: &Generator,
        _combinator: &Combinator,
    ) -> Option<Rocket> {
        //if the planet can't build rockets, you're screwed
        //LOG
        let mut payload = Payload::new();
        let mut payload_ris = Payload::new();
        payload.insert("Message".to_string(), "Asteroid".to_string());
        payload_ris.insert("Response to".to_string(), "Asteroid".to_string());
        //LOG
        // TODO add detailed (debug) logging
        let mut ris = None;
        if !state.can_have_rocket() {
            ris = None;
        }
        //if you've already got a rocket ready, use it!
        else if state.has_rocket() {
            ris = state.take_rocket();
        }
        //try to build a rocket if you have any energy left
        else if let Some(idx) = get_charged_cell_index() {
            match state.build_rocket(idx as usize) {
                Ok(_) => {
                    push_free_cell(idx);
                    println!("Used a charged cell at index {}, to build a rocket", idx);
                    return state.take_rocket();
                }
                //build failed, log the error and return none
                Err(err) => {
                    push_charged_cell(idx);
                    //TODO change this to log
                    println!("{}", err);
                    return None;
                }
            }
        }
        if ris.is_none() {
            payload_ris.insert("Result".to_string(), "no rocket available".to_string());
        } else {
            payload_ris.insert("Result".to_string(), "a rocket is available".to_string());
        }

        //LOG

        let event = LogEvent::new(
            ActorType::Orchestrator,
            0u64,
            ActorType::Planet,
            state.id().clone().to_string(),
            MessageOrchestratorToPlanet,
            RCV_MSG_LOG_CHNL,
            payload,
        );

        log_msg!(event, RCV_MSG_LOG_CHNL);

        let event_ris = LogEvent::new(
            ActorType::Planet,
            state.id(),
            ActorType::Orchestrator,
            "0".to_string(),
            MessagePlanetToOrchestrator,
            ACK_MSG_LOG_CHNL,
            payload_ris,
        );

        log_msg!(event_ris, ACK_MSG_LOG_CHNL);

        //LOG

        ris
        //shouldn't be able to get here, but just in case...
        //None
    }

    fn start(&mut self, state: &PlanetState) {
        //println!("Planet {} AI started", state.id());
        let mut payload = Payload::new();
        payload.insert("Message".to_string(), "Planet AI start".to_string());
        let event = LogEvent::new(
            ActorType::Orchestrator,
            0u64,
            ActorType::Planet,
            state.id().to_string(),
            MessageOrchestratorToPlanet,
            RCV_MSG_LOG_CHNL,
            payload,
        );
        log_msg!(event, RCV_MSG_LOG_CHNL);
    }

    fn stop(&mut self, _state: &PlanetState) {
        let mut payload = Payload::new();
        payload.insert("Message".to_string(), "Planet AI stop".to_string());
        let event = LogEvent::new(
            ActorType::Orchestrator,
            0u64,
            ActorType::Planet,
            _state.id().to_string(),
            MessageOrchestratorToPlanet,
            RCV_MSG_LOG_CHNL,
            payload,
        );
        log_msg!(event, RCV_MSG_LOG_CHNL);
    }
}

pub trait ResToString {
    fn res_to_string(&self) -> String;
}

impl ResToString for BasicResourceType {
    fn res_to_string(&self) -> String {
        match self {
            BasicResourceType::Carbon => String::from("carbon"),
            BasicResourceType::Hydrogen => String::from("hydrogen"),
            BasicResourceType::Oxygen => String::from("oxygen"),
            BasicResourceType::Silicon => String::from("silicon"),
        }
    }
}
impl ResToString for ComplexResourceType {
    fn res_to_string(&self) -> String {
        match self {
            ComplexResourceType::AIPartner => String::from("AIPartner"),
            ComplexResourceType::Diamond => String::from("Diamond"),
            ComplexResourceType::Life => String::from("Life"),
            ComplexResourceType::Robot => String::from("Robot"),
            ComplexResourceType::Water => String::from("Water"),
            ComplexResourceType::Dolphin => String::from("Dolphin"),
        }
    }
}

pub const N_CELLS: usize = 5; // TODO da cambiare in base al pianeta
pub mod stacks {
    use crate::N_CELLS;
    use std::sync::Mutex;

    pub(crate) static FREE_CELL_STACK: Mutex<Vec<u32>> = Mutex::new(Vec::new());
    pub(crate) static CHARGED_CELL_STACK: Mutex<Vec<u32>> = Mutex::new(Vec::new());
    pub fn initialize_free_cell_stack() {
        //initialize the free cell stack with all the possible indexes
        let free_cell_stack = FREE_CELL_STACK.lock();
        match free_cell_stack {
            Ok(mut vec) => {
                //empty previous values in case of reset
                vec.clear();
                for i in 0..N_CELLS {
                    vec.push(i as u32);
                }
                //put the indexes in the correct orientation
                vec.reverse();
            }
            Err(err) => {
                println!("{}", err);
            }
        }

        //same thing as above but we just make sure that the vector is empty
        let charged_cell_stack = CHARGED_CELL_STACK.lock();
        match charged_cell_stack {
            Ok(mut vec) => {
                vec.clear();
            }
            Err(err) => {
                println!("{}", err);
            }
        }
    }

    pub fn get_free_cell_index() -> Option<u32> {
        let free_cell_stack = FREE_CELL_STACK.lock();
        match free_cell_stack {
            Ok(mut vec) => vec.pop(),
            Err(err) => {
                println!("{}", err);
                None
            }
        }
    }

    pub fn get_charged_cell_index() -> Option<u32> {
        let charged_cell_stack = CHARGED_CELL_STACK.lock();
        match charged_cell_stack {
            Ok(mut vec) => vec.pop(),
            Err(err) => {
                println!("{}", err);
                None
            }
        }
    }
    pub fn push_free_cell(index: u32) {
        let free_cell_stack = FREE_CELL_STACK.lock();
        match free_cell_stack {
            Ok(mut vec) => {
                if vec.len() < N_CELLS {
                    vec.push(index);
                }
            }
            Err(err) => {
                println!("{}", err);
            }
        }
    }
    pub fn push_charged_cell(index: u32) {
        let charged_cell_stack = CHARGED_CELL_STACK.lock();
        match charged_cell_stack {
            Ok(mut vec) => {
                if vec.len() < N_CELLS {
                    vec.push(index);
                }
            }
            Err(err) => {
                println!("{}", err);
            }
        }
    }

    pub fn peek_charged_cell_index() -> Option<u32> {
        let charged_cell_stack = CHARGED_CELL_STACK.lock();
        match charged_cell_stack {
            Ok(vec) => vec.last().copied(),
            Err(err) => {
                println!("{}", err);
                None
            }
        }
    }
}
