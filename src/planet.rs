use common_game::components::planet::{Planet, PlanetAI, PlanetState, PlanetType};
use common_game::components::resource::BasicResourceType::*;
use common_game::components::resource::{
    BasicResource, BasicResourceType, Combinator, ComplexResource, ComplexResourceRequest,
    ComplexResourceType, Generator, GenericResource,
};
use common_game::components::rocket::Rocket;
use common_game::protocols::messages::{
    ExplorerToPlanet, OrchestratorToPlanet, PlanetToExplorer, PlanetToOrchestrator,
};
use crossbeam_channel::{Receiver, Sender};

use common_game::logging::EventType::{
    MessageOrchestratorToPlanet, MessagePlanetToExplorer, MessagePlanetToOrchestrator,
};
use common_game::logging::{ActorType, Channel, EventType, LogEvent, Payload};
use stacks::{
    get_charged_cell_index, get_free_cell_index, initialize_free_cell_stack, push_charged_cell,
    push_free_cell,
};
///////////////////////////////////////////////////////////////////////////////////////////
// CrabRave Constructor
///////////////////////////////////////////////////////////////////////////////////////////
const RCV_MSG_LOG_CHNL: Channel = Channel::Info; // change this 2 in order to change the channel of the logs
const ACK_MSG_LOG_CHNL: Channel = Channel::Info;
const ERR_LOG_CHNL: Channel = Channel::Error;
const DEBUG_LOG_CHNL: Channel = Channel::Debug;
const INTRNL_ACTN_LOG_CHNL: Channel = Channel::Info;
const TRACE_LOG_CHNL: Channel = Channel::Trace;
const WARN_LOG_CHNL: Channel = Channel::Warning;

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
#[macro_export]
macro_rules! create_internal_log_msg {
    ($id:expr, $channel:expr $(,$a:expr, $b:expr)* $(,)?) => {{
        let mut payload=Payload::new();
        $(
            payload.insert($a, $b);
        )*
        let event_deb = LogEvent::new(
            ActorType::Planet,
            $id,
            ActorType::Planet,
            $id.to_string(),
            EventType::InternalPlanetAction,
            $channel,
            payload,
        );
        log_msg!(event_deb, $channel);
    }};
}
#[macro_export]
macro_rules! create_internal_action_log_msg {
    ($payload:expr, $id:expr) => {{
        let event_deb = LogEvent::new(
            ActorType::Planet,
            $id,
            ActorType::Planet,
            $id.to_string(),
            EventType::InternalPlanetAction,
            DEBUG_LOG_CHNL,
            $payload,
        );
        log_msg!(event_deb, DEBUG_LOG_CHNL);
    }};
}
//This function will be called by the Orchestrator
pub fn create_planet(
    rx_orchestrator: Receiver<OrchestratorToPlanet>,
    tx_orchestrator: Sender<PlanetToOrchestrator>,
    rx_explorer: Receiver<ExplorerToPlanet>,
    planet_id: u32,
) -> Result<Planet, String> {
    let (planet_type, ai, gen_rules, comb_rules, orchestrator_channels, explorer_channels) = (
        PlanetType::D,
        OneMillionCrabs::new(),
        vec![Carbon, Hydrogen, Oxygen, Silicon],
        vec![],
        (rx_orchestrator, tx_orchestrator),
        rx_explorer,
    );

    //LOG
    let mut payload = Payload::new();
    payload.insert(
        String::from("gen_rules"),
        gen_rules.iter().map(|x| x.to_string_2() + ", ").collect(),
    );
    payload.insert("Message".to_string(), "New planet created".to_string());
    //LOG

    let new_planet = Planet::new(
        planet_id,
        planet_type,
        Box::new(ai),
        gen_rules,
        comb_rules,
        orchestrator_channels,
        explorer_channels,
    )?;

    //LOG
    let event = LogEvent::new(
        ActorType::Planet,
        planet_id,
        ActorType::Planet,
        planet_id.to_string(),
        EventType::InternalPlanetAction,
        INTRNL_ACTN_LOG_CHNL,
        payload,
    );
    log_msg!(event, INTRNL_ACTN_LOG_CHNL);
    //LOG

    Ok(new_planet)
}

///////////////////////////////////////////////////////////////////////////////////////////
// PlanetAI
///////////////////////////////////////////////////////////////////////////////////////////

pub struct OneMillionCrabs;

impl OneMillionCrabs {
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
            INTRNL_ACTN_LOG_CHNL,
            payload,
        );
        log_msg!(event, INTRNL_ACTN_LOG_CHNL);
        //LOG
        initialize_free_cell_stack(0u64);
        Self
    }
}

impl PlanetAI for OneMillionCrabs {
    fn handle_orchestrator_msg(
        &mut self,
        state: &mut PlanetState,
        _generator: &Generator,
        _combinator: &Combinator,
        msg: OrchestratorToPlanet,
    ) -> Option<PlanetToOrchestrator> {
        //LOG
        let mut payload_deb = Payload::new();
        payload_deb.insert("Message".to_string(), "handle_orchestrator_msg".to_string());
        payload_deb.insert(
            "Data".to_string(),
            format!(
                "planet state: {:?}, msg: {:?}",
                PlanetState::to_dummy(state),
                msg.to_string_2()
            ),
        );
        let event_2 = LogEvent::new(
            ActorType::Planet,
            state.id(),
            ActorType::Planet,
            state.id().to_string(),
            EventType::InternalPlanetAction,
            DEBUG_LOG_CHNL,
            payload_deb,
        );
        log_msg!(event_2, DEBUG_LOG_CHNL);
        //LOG

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
                if let Some(idx) = get_free_cell_index(state.id() as u64) {
                    //LOG
                    let mut payload_deb = Payload::new();
                    payload_deb.insert("Action".to_string(), "get_free_cell_index".to_string());
                    payload_deb.insert("Result".to_string(), format!("Some({})", idx));
                    let event_deb = LogEvent::new(
                        ActorType::Planet,
                        state.id(),
                        ActorType::Planet,
                        state.id().to_string(),
                        EventType::InternalPlanetAction,
                        DEBUG_LOG_CHNL,
                        payload_deb,
                    );
                    log_msg!(event_deb, DEBUG_LOG_CHNL);
                    //LOG

                    state.cell_mut(idx as usize).charge(sunray);
                    push_charged_cell(idx, state.id() as u64);

                    //LOG
                    let mut payload_deb = Payload::new();
                    payload_deb.insert(
                        "Action".to_string(),
                        "cell_mut(index).charge(sunray)".to_string(),
                    );
                    payload_deb.insert("Data".to_string(), format!("index: {}", idx));
                    let event_deb = LogEvent::new(
                        ActorType::Planet,
                        state.id(),
                        ActorType::Planet,
                        state.id().to_string(),
                        EventType::InternalPlanetAction,
                        DEBUG_LOG_CHNL,
                        payload_deb,
                    );
                    log_msg!(event_deb, DEBUG_LOG_CHNL);
                    //LOG

                    payload_ris.insert("Message".to_string(), "SunrayAck".to_string());
                    payload_ris.insert(String::from("Result"), String::from("EnergyCell charged"));
                    payload_ris.insert(String::from("EnergyCell index"), format!("{}", idx));
                    payload_ris.insert(
                        String::from("Response data"),
                        format!("planet_id: {}", state.id()),
                    );
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

                Some(PlanetToOrchestrator::SunrayAck {
                    planet_id: state.id(),
                })
            }
            _ => {
                //LOG TODO add more information
                let mut payload = Payload::new();
                payload.insert(
                    String::from("Message"),
                    "message behaviour not defined".to_string(),
                );
                payload.insert(
                    "Data".to_string(),
                    format!(
                        "planet state: {:?}, msg: {:?}",
                        PlanetState::to_dummy(state),
                        msg.to_string_2()
                    ),
                );
                let event = LogEvent::new(
                    ActorType::Orchestrator,
                    0u64,
                    ActorType::Planet,
                    state.id().to_string(),
                    EventType::MessageOrchestratorToPlanet,
                    ERR_LOG_CHNL,
                    payload,
                );
                log_msg!(event, ERR_LOG_CHNL);
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
        //LOG
        let mut payload_deb = Payload::new();
        payload_deb.insert("Message".to_string(), "handle_orchestrator_msg".to_string());
        payload_deb.insert(
            "Data".to_string(),
            format!(
                "planet state: {:?}, msg: {:?}",
                PlanetState::to_dummy(state),
                msg.to_string_2()
            ),
        );
        let event_deb = LogEvent::new(
            ActorType::Planet,
            state.id(),
            ActorType::Planet,
            state.id().to_string(),
            EventType::InternalPlanetAction,
            DEBUG_LOG_CHNL,
            payload_deb,
        );
        log_msg!(event_deb, DEBUG_LOG_CHNL);
        //LOG

        match msg {
            ExplorerToPlanet::AvailableEnergyCellRequest { explorer_id: id } => {
                // restituisce la prima cell carica, se c'è

                let mut payload_ris = Payload::new();
                //add debug LOGS
                let mut n_available_cells = 0;
                for i in 0..N_CELLS {
                    if state.cell(i).is_charged() {
                        n_available_cells += 1;
                    }
                }

                payload_ris.insert(
                    "Message".to_string(),
                    "AvailableEnergyCellResponse".to_string(),
                );

                payload_ris.insert(String::from("Result"), "EnergyCell available".to_string());

                payload_ris.insert(
                    String::from("EnergyCell number"),
                    format!("{}", n_available_cells),
                );

                let ris = Some(PlanetToExplorer::AvailableEnergyCellResponse {
                    available_cells: n_available_cells,
                });

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

                create_internal_log_msg!(
                    state.id(),
                    DEBUG_LOG_CHNL,
                    "Action".to_string(),
                    "generator.all_available_recipes()".to_string()
                );
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

                create_internal_log_msg!(
                    state.id(),
                    DEBUG_LOG_CHNL,
                    "Action".to_string(),
                    "combinator.all_available_recipes()".to_string()
                );
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
                let mut res = Some(PlanetToExplorer::GenerateResourceResponse { resource: None });
                let mut res_type = false;
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

                //LOG
                let requested_resource = resource;
                // controllo se c'è una cella carica

                if let Some(cell_idx) = get_charged_cell_index(state.id() as u64) {
                    //LOG
                    let mut payload_deb2 = Payload::new();
                    //LOG

                    // se c'è una cella carica
                    // ottengo la cella da passare al generator
                    let cell = state.cell_mut(cell_idx as usize);
                    // pattern matching per generare la risorsa corretta
                    let generated_resource = match requested_resource {
                        BasicResourceType::Carbon => {
                            payload_deb2.insert(
                                "Action".to_string(),
                                "generator.make_carbon()".to_string(),
                            );
                            generator.make_carbon(cell).map(BasicResource::Carbon)
                        } // make_ controlla già se la risorsa è presente in generator
                        BasicResourceType::Silicon => {
                            payload_deb2.insert(
                                "Action".to_string(),
                                "generator.make_silicon()".to_string(),
                            );
                            generator.make_silicon(cell).map(BasicResource::Silicon)
                        }
                        BasicResourceType::Oxygen => {
                            payload_deb2.insert(
                                "Action".to_string(),
                                "generator.make_oxygen()".to_string(),
                            );
                            generator.make_oxygen(cell).map(BasicResource::Oxygen)
                        }
                        BasicResourceType::Hydrogen => {
                            payload_deb2.insert(
                                "Action".to_string(),
                                "generator.make_hydrogen()".to_string(),
                            );
                            generator.make_hydrogen(cell).map(BasicResource::Hydrogen)
                        }
                    };

                    //LOG
                    payload_deb2.insert("Result".to_string(), format!("{:?}", generated_resource));
                    create_internal_action_log_msg!(payload_deb2, state.id());
                    //LOG

                    // verifico il risultato di state.generator.make...
                    match generated_resource {
                        Ok(resource) => {
                            //LOG
                            payload_ris.insert(
                                "Mesage".to_string(),
                                "Generated Resource Response".to_string(),
                            );

                            payload_ris.insert(
                                "Result".to_string(),
                                format!("produced resource: {:?}", resource),
                            );
                            //LOG
                            push_free_cell(cell_idx, state.id() as u64);
                            res_type = true;
                            res = Some(PlanetToExplorer::GenerateResourceResponse {
                                resource: Some(resource),
                            });
                        }
                        Err(err) => {
                            //LOG
                            create_internal_log_msg!(
                                state.id(),
                                ERR_LOG_CHNL,
                                "ERR".to_string(),
                                format!("{:?}", err)
                            );
                            //LOG
                            push_charged_cell(cell_idx, state.id() as u64);
                        }
                    }
                }
                //LOG
                if !res_type {
                    payload_ris.insert(
                        String::from("Response to"),
                        "Generated resource request".to_string(),
                    );

                    payload_ris.insert(
                        String::from("Result"),
                        format!("resource: {:?} not produced", resource),
                    );
                }

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
                res
            }
            //TODO use explorer_id to send the gen resource to correct Explorer
            ExplorerToPlanet::CombineResourceRequest {
                explorer_id,
                msg: resource,
            } => {
                //renamed msg to resouce to be more consistent with generateresourcerequest
                // searching the index of the first free cell

                let res;

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

                if let Some(cell_idx) = get_charged_cell_index(state.id() as u64) {
                    //LOG
                    let mut payload_deb2 = Payload::new();
                    //LOG

                    let cell = state.cell_mut(cell_idx as usize);
                    let complex_resource: Result<
                        ComplexResource,
                        (String, GenericResource, GenericResource),
                    > = match resource {
                        ComplexResourceRequest::Water(r1, r2) => {
                            payload_deb2.insert(
                                "Action".to_string(),
                                format!("combinator.make_water({}, {}, {})", r1, r2, cell_idx),
                            );

                            combinator
                                .make_water(r1, r2, cell)
                                .map(ComplexResource::Water)
                                .map_err(|(e, r1, r2)| {
                                    (
                                        e,
                                        GenericResource::BasicResources(BasicResource::Hydrogen(
                                            r1,
                                        )),
                                        GenericResource::BasicResources(BasicResource::Oxygen(r2)),
                                    )
                                })
                        }
                        ComplexResourceRequest::Diamond(r1, r2) => {
                            payload_deb2.insert(
                                "Action".to_string(),
                                format!("combinator.make_diamond({}, {}, {})", r1, r2, cell_idx),
                            );
                            combinator
                                .make_diamond(r1, r2, cell)
                                .map(ComplexResource::Diamond)
                                .map_err(|(e, r1, r2)| {
                                    (
                                        e,
                                        GenericResource::BasicResources(BasicResource::Carbon(r1)),
                                        GenericResource::BasicResources(BasicResource::Carbon(r2)),
                                    )
                                })
                        }
                        ComplexResourceRequest::Life(r1, r2) => {
                            payload_deb2.insert(
                                "Action".to_string(),
                                format!("combinator.make_life({}, {}, {})", r1, r2, cell_idx),
                            );
                            combinator
                                .make_life(r1, r2, cell)
                                .map(ComplexResource::Life)
                                .map_err(|(e, r1, r2)| {
                                    (
                                        e,
                                        GenericResource::ComplexResources(ComplexResource::Water(
                                            r1,
                                        )),
                                        GenericResource::BasicResources(BasicResource::Carbon(r2)),
                                    )
                                })
                        }

                        ComplexResourceRequest::Robot(r1, r2) => {
                            payload_deb2.insert(
                                "Action".to_string(),
                                format!("combinator.make_robot({}, {}, {})", r1, r2, cell_idx),
                            );
                            combinator
                                .make_robot(r1, r2, cell)
                                .map(ComplexResource::Robot)
                                .map_err(|(e, r1, r2)| {
                                    (
                                        e,
                                        GenericResource::BasicResources(BasicResource::Silicon(r1)),
                                        GenericResource::ComplexResources(ComplexResource::Life(
                                            r2,
                                        )),
                                    )
                                })
                        }

                        ComplexResourceRequest::Dolphin(r1, r2) => {
                            payload_deb2.insert(
                                "Action".to_string(),
                                format!("combinator.make_dolphin({}, {}, {})", r1, r2, cell_idx),
                            );
                            combinator
                                .make_dolphin(r1, r2, cell)
                                .map(ComplexResource::Dolphin)
                                .map_err(|(e, r1, r2)| {
                                    (
                                        e,
                                        GenericResource::ComplexResources(ComplexResource::Water(
                                            r1,
                                        )),
                                        GenericResource::ComplexResources(ComplexResource::Life(
                                            r2,
                                        )),
                                    )
                                })
                        }

                        ComplexResourceRequest::AIPartner(r1, r2) => {
                            payload_deb2.insert(
                                "Action".to_string(),
                                format!("combinator.make_aipartner({}, {}, {})", r1, r2, cell_idx),
                            );
                            combinator
                                .make_aipartner(r1, r2, cell)
                                .map(ComplexResource::AIPartner)
                                .map_err(|(e, r1, r2)| {
                                    (
                                        e,
                                        GenericResource::ComplexResources(ComplexResource::Robot(
                                            r1,
                                        )),
                                        GenericResource::ComplexResources(
                                            ComplexResource::Diamond(r2),
                                        ),
                                    )
                                })
                        }
                    };

                    //LOG
                    payload_deb2.insert("Result".to_string(), format!("{:?}", complex_resource));
                    create_internal_action_log_msg!(payload_deb2, state.id());
                    //LOG

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
                            //LOG

                            push_free_cell(cell_idx, state.id() as u64);
                            res = Some(PlanetToExplorer::CombineResourceResponse {
                                complex_response: Ok(resource),
                            });
                        }
                        Err(err) => {
                            push_charged_cell(cell_idx, state.id() as u64);
                            //LOG
                            payload_ris.insert(
                                "Message".to_string(),
                                "Combine resource response".to_string(),
                            );

                            payload_ris.insert("Result".to_string(), format!("{:?}", err));

                            let mut payload_deb2 = Payload::new();
                            payload_deb2.insert("ERR".to_string(), format!("{:?}", err));
                            let event_deb2 = LogEvent::new(
                                ActorType::Planet,
                                state.id(),
                                ActorType::Planet,
                                state.id().to_string(),
                                EventType::InternalPlanetAction,
                                ERR_LOG_CHNL,
                                payload_deb2,
                            );
                            log_msg!(event_deb2, ERR_LOG_CHNL);

                            //LOG

                            res = Some(PlanetToExplorer::CombineResourceResponse {
                                complex_response: Err(err),
                            });
                        }
                    }
                } else {
                    //LOG
                    create_internal_log_msg!(
                        state.id(),
                        ERR_LOG_CHNL,
                        "ERR".to_string(),
                        "No available cell found, Explorer MUST make sure there are charged cells"
                            .to_string()
                    );
                    //LOG

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

                    //LOG

                    res = Some(PlanetToExplorer::CombineResourceResponse {
                        complex_response: Err(("no available cell".to_string(), ret1, ret2)),
                    });
                }

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

                res
            }
        }
    }

    /// Handler used to determine the strategy in case of an incoming asteroid.
    /// It will usually try to build a rocket if it can and if it has any
    /// energy cells available.
    /// As for our planet, it's a type D, so the planet will ALWAYS die
    /// when it gets an asteroid. Any other behavior is unexpected and
    /// should be reported.
    /// Refer to the common crate documentation for more info on the
    /// default behavior of this function.
    fn handle_asteroid(
        &mut self,
        state: &mut PlanetState,
        _generator: &Generator,
        _combinator: &Combinator,
    ) -> Option<Rocket> {
        //if the planet can't build rockets, you're screwed

        //LOG
        let mut payload_deb = Payload::new();
        payload_deb.insert("Message".to_string(), "handle_orchestrator_msg".to_string());
        payload_deb.insert(
            "Data".to_string(),
            format!("planet state: {:?}", PlanetState::to_dummy(state)),
        );
        let event_deb = LogEvent::new(
            ActorType::Planet,
            state.id(),
            ActorType::Planet,
            state.id().to_string(),
            EventType::InternalPlanetAction,
            DEBUG_LOG_CHNL,
            payload_deb,
        );
        log_msg!(event_deb, DEBUG_LOG_CHNL);

        let mut payload = Payload::new();
        let mut payload_ris = Payload::new();
        payload.insert("Message".to_string(), "Asteroid".to_string());
        payload_ris.insert("Response to".to_string(), "Asteroid".to_string());

        let mut payload_deb = Payload::new();
        payload_deb.insert("Action".to_string(), "can_have_rocket()".to_string());
        payload_deb.insert(
            "Response".to_string(),
            format!("{}", state.can_have_rocket()),
        );
        create_internal_action_log_msg!(payload_deb, state.id());

        //LOG

        let mut ris = None;
        if !state.can_have_rocket() {
            ris = None;
        }
        //if you've already got a rocket ready, use it!
        else {
            //LOG
            let mut payload_deb = Payload::new();
            payload_deb.insert("Action".to_string(), "has_rocket()".to_string());
            payload_deb.insert("Response".to_string(), format!("{}", state.has_rocket()));
            create_internal_action_log_msg!(payload_deb, state.id());
            //LOG

            if state.has_rocket() {
                //LOG
                let mut payload_deb = Payload::new();
                payload_deb.insert("Action".to_string(), "take_rocket()".to_string());
                create_internal_action_log_msg!(payload_deb, state.id());
                //LOG
                ris = state.take_rocket();
            }
            //try to build a rocket if you have any energy left
            else {
                //LOG
                let mut payload_deb = Payload::new();
                payload_deb.insert("Action".to_string(), "get_charged_cell_index()".to_string());
                //LOG

                if let Some(idx) = get_charged_cell_index(state.id() as u64) {
                    //LOG
                    payload_deb.insert("Response".to_string(), format!("Some({})", idx));
                    create_internal_action_log_msg!(payload_deb, state.id());

                    let mut payload_deb2 = Payload::new();
                    payload_deb2.insert("Action".to_string(), format!("build_rocket({})", idx));
                    //LOG

                    match state.build_rocket(idx as usize) {
                        Ok(_) => {
                            //LOG
                            payload_deb2.insert("Response".to_string(), "Ok".to_string());
                            create_internal_action_log_msg!(payload_deb2, state.id());
                            //LOG

                            push_free_cell(idx, state.id() as u64);
                            //println!("Used a charged cell at index {}, to build a rocket", idx);
                            ris = state.take_rocket();
                        }
                        //build failed, log the error and return none
                        Err(err) => {
                            //LOG
                            payload_deb2.insert("Response".to_string(), "Err".to_string());
                            create_internal_action_log_msg!(payload_deb2, state.id());

                            create_internal_log_msg!(
                                state.id(),
                                ERR_LOG_CHNL,
                                "ERR".to_string(),
                                format!("{}", err)
                            );
                            //LOG
                            push_charged_cell(idx, state.id() as u64);
                            ris = None;
                        }
                    }
                } else {
                    //LOG
                    payload_deb.insert("Response".to_string(), "None".to_string());
                    create_internal_action_log_msg!(payload_deb, state.id());
                    //LOG
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
            state.id().to_string(),
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

pub trait ToString2 {
    fn to_string_2(&self) -> String;
}

impl ToString2 for BasicResourceType {
    fn to_string_2(&self) -> String {
        match self {
            BasicResourceType::Carbon => String::from("carbon"),
            BasicResourceType::Hydrogen => String::from("hydrogen"),
            BasicResourceType::Oxygen => String::from("oxygen"),
            BasicResourceType::Silicon => String::from("silicon"),
        }
    }
}
impl ToString2 for ComplexResourceType {
    fn to_string_2(&self) -> String {
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
impl ToString2 for OrchestratorToPlanet {
    fn to_string_2(&self) -> String {
        match self {
            OrchestratorToPlanet::InternalStateRequest => String::from("InternalStateRequest"),
            OrchestratorToPlanet::Sunray(_) => String::from("Sunray"),
            OrchestratorToPlanet::Asteroid(_) => String::from("Asteroid"),
            OrchestratorToPlanet::StartPlanetAI => String::from("StartPlanetAI"),
            OrchestratorToPlanet::StopPlanetAI => String::from("StopPlanetAI"),
            OrchestratorToPlanet::KillPlanet => String::from("KillPlanet"),
            OrchestratorToPlanet::IncomingExplorerRequest { .. } => {
                String::from("IncomingExplorerRequest")
            }
            OrchestratorToPlanet::OutgoingExplorerRequest { .. } => {
                String::from("OutgoingExplorerRequest")
            }
        }
    }
}
impl ToString2 for ExplorerToPlanet {
    fn to_string_2(&self) -> String {
        match self {
            ExplorerToPlanet::SupportedResourceRequest { .. } => {
                String::from("SupportedResourceRequest")
            }
            ExplorerToPlanet::SupportedCombinationRequest { .. } => {
                String::from("SupportedCombinationRequest")
            }
            ExplorerToPlanet::GenerateResourceRequest { .. } => {
                String::from("GenerateResourceRequest")
            }
            ExplorerToPlanet::CombineResourceRequest { .. } => {
                String::from("CombineResourceRequest")
            }
            ExplorerToPlanet::AvailableEnergyCellRequest { .. } => {
                String::from("AvailableEnergyCellRequest")
            }
        }
    }
}

pub const N_CELLS: usize = 5;

/// Module used to implement an energy cell management system based on a stack.
/// Provides O(1) lookups, charges and discharges.
mod stacks {
    use crate::N_CELLS;
    use crate::planet::{DEBUG_LOG_CHNL, ERR_LOG_CHNL, TRACE_LOG_CHNL, WARN_LOG_CHNL};
    use common_game::logging::{ActorType, Channel, EventType, LogEvent, Payload};
    use std::sync::Mutex;

    pub(crate) static FREE_CELL_STACK: Mutex<Vec<u32>> = Mutex::new(Vec::new());
    pub(crate) static CHARGED_CELL_STACK: Mutex<Vec<u32>> = Mutex::new(Vec::new());

    /// Initializes the internal vectors used to handle the stack.
    /// MUST be called everytime the planet is created, for example at
    /// the start of PlanetAI.
    pub fn initialize_free_cell_stack(planet_id: u64) {
        //initialize the free cell stack with all the possible indexes

        //LOG
        create_internal_log_msg!(
            planet_id,
            DEBUG_LOG_CHNL,
            "Action".to_string(),
            "initialize_free_cell_stack".to_string()
        );

        create_internal_log_msg!(
            planet_id,
            TRACE_LOG_CHNL,
            "Action".to_string(),
            "FREE_CELL_STACK.lock()".to_string()
        );
        //LOG
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
                //LOG
                create_internal_log_msg!(
                    planet_id,
                    ERR_LOG_CHNL,
                    "Action".to_string(),
                    "FREE_CELL_STACK.lock()".to_string(),
                    "ERR".to_string(),
                    format!("{:?}", err)
                );
                //LOG
            }
        }

        //same thing as above but we just make sure that the vector is empty
        //LOG
        create_internal_log_msg!(
            planet_id,
            TRACE_LOG_CHNL,
            "Action".to_string(),
            "CHARGED_CELL_STACK.lock()".to_string()
        );
        //LOG
        let charged_cell_stack = CHARGED_CELL_STACK.lock();
        match charged_cell_stack {
            Ok(mut vec) => {
                vec.clear();
            }
            Err(err) => {
                //LOG
                create_internal_log_msg!(
                    planet_id,
                    ERR_LOG_CHNL,
                    "Action".to_string(),
                    "CHARGED_CELL_STACK.lock()".to_string(),
                    "ERR".to_string(),
                    format!("{:?}", err)
                );
                //LOG
            }
        }
    }

    /// Pulls out a free cell from the corresponding stack.
    /// returns Some and the correspnding index to charge
    /// or None if there are no available cells
    pub fn get_free_cell_index(planet_id: u64) -> Option<u32> {
        let free_cell_stack = FREE_CELL_STACK.lock();
        //LOG
        create_internal_log_msg!(
            planet_id,
            TRACE_LOG_CHNL,
            "Action".to_string(),
            "FREE_CELL_STACK.lock()".to_string()
        );
        //LOG
        let res = match free_cell_stack {
            Ok(mut vec) => {
                //LOG
                create_internal_log_msg!(
                    planet_id,
                    TRACE_LOG_CHNL,
                    "Action".to_string(),
                    "free_cell_stack.pop()".to_string()
                );
                //LOG
                vec.pop()
            }
            Err(err) => {
                //LOG
                create_internal_log_msg!(
                    planet_id,
                    ERR_LOG_CHNL,
                    "Action".to_string(),
                    "get_free_cell_index".to_string(),
                    "ERR".to_string(),
                    format!("{:?}", err)
                );
                //LOG
                None
            }
        };

        //LOG
        create_internal_log_msg!(
            planet_id,
            DEBUG_LOG_CHNL,
            "Action".to_string(),
            "get_free_cell_index".to_string(),
            "Result".to_string(),
            format!("{:?}", res)
        );
        //LOG

        res
    }

    /// Pulls out a charged cell from the corresponding stack.
    /// returns Some and the correspnding index to discharge
    /// or None if there are no available cells
    pub fn get_charged_cell_index(planet_id: u64) -> Option<u32> {
        let charged_cell_stack = CHARGED_CELL_STACK.lock();
        //LOG
        create_internal_log_msg!(
            planet_id,
            TRACE_LOG_CHNL,
            "Action".to_string(),
            "CHARGED_CELL_STACK.lock()".to_string()
        );
        //LOG
        let res;
        match charged_cell_stack {
            Ok(mut vec) => {
                res = vec.pop();
                //LOG
                create_internal_log_msg!(
                    planet_id,
                    TRACE_LOG_CHNL,
                    "Action".to_string(),
                    "charged_cell_stack.pop()".to_string()
                );
                //LOG
            }
            Err(err) => {
                //LOG
                create_internal_log_msg!(
                    planet_id,
                    ERR_LOG_CHNL,
                    "Action".to_string(),
                    "get_charged_cell_index".to_string(),
                    "ERR".to_string(),
                    format!("{:?}", err)
                );
                //LOG
                res = None;
            }
        }

        //LOG
        create_internal_log_msg!(
            planet_id,
            DEBUG_LOG_CHNL,
            "Action".to_string(),
            "get_charged_cell_index".to_string(),
            "Result".to_string(),
            format!("{:?}", res)
        );
        //LOG
        res
    }

    /// Pushes a free energy cell back into the stack.
    /// The user must verify that there is available space,
    /// as the function will otherwise give no output without
    /// increasing the available space.
    pub fn push_free_cell(index: u32, planet_id: u64) {
        //LOG
        create_internal_log_msg!(
            planet_id,
            DEBUG_LOG_CHNL,
            "Action".to_string(),
            "push_free_cell".to_string(),
            "index".to_string(),
            format!("{:?}", index)
        );
        //LOG
        let free_cell_stack = FREE_CELL_STACK.lock();
        //LOG
        create_internal_log_msg!(
            planet_id,
            TRACE_LOG_CHNL,
            "Action".to_string(),
            "FREE_CELL_STACK.lock()".to_string()
        );
        //LOG

        match free_cell_stack {
            Ok(mut vec) => {
                if vec.len() < N_CELLS {
                    vec.push(index);
                    //LOG
                    create_internal_log_msg!(
                        planet_id,
                        TRACE_LOG_CHNL,
                        "Action".to_string(),
                        format!("free_cell_stack.push({})", index)
                    );
                    //LOG
                } else {
                    //LOG
                    create_internal_log_msg!(
                        planet_id,
                        WARN_LOG_CHNL,
                        "Action".to_string(),
                        format!("free_cell_stack.push({})", index),
                        "WARN".to_string(),
                        format!("free_cell_stack.len()({})>=N_CELLS({})", vec.len(), N_CELLS)
                    );
                    //LOG
                }
            }
            Err(err) => {
                //LOG
                create_internal_log_msg!(
                    planet_id,
                    ERR_LOG_CHNL,
                    "Action".to_string(),
                    "push_free_cell".to_string(),
                    "ERR".to_string(),
                    format!("{:?}", err)
                );
                //LOG
            }
        }
    }

    /// Pushes a free energy cell back into the stack.
    /// The user must verify that the maximum size hasn't already
    /// been reached, as the function will otherwise give
    /// no output without increasing the available space.
    pub fn push_charged_cell(index: u32, planet_id: u64) {
        //LOG
        create_internal_log_msg!(
            planet_id,
            DEBUG_LOG_CHNL,
            "Action".to_string(),
            "push_charged_cell".to_string(),
            "index".to_string(),
            format!("{:?}", index)
        );
        //LOG
        let charged_cell_stack = CHARGED_CELL_STACK.lock();
        //LOG
        create_internal_log_msg!(
            planet_id,
            TRACE_LOG_CHNL,
            "Action".to_string(),
            "CHARGED_CELL_STACK.lock()".to_string()
        );
        //LOG
        match charged_cell_stack {
            Ok(mut vec) => {
                if vec.len() < N_CELLS {
                    vec.push(index);
                    //LOG
                    create_internal_log_msg!(
                        planet_id,
                        TRACE_LOG_CHNL,
                        "Action".to_string(),
                        format!("charged_cell_stack.push({})", index)
                    );
                    //LOG
                } else {
                    //LOG
                    create_internal_log_msg!(
                        planet_id,
                        WARN_LOG_CHNL,
                        "Action".to_string(),
                        format!("charged_cell_stack.push({})", index),
                        "WARN".to_string(),
                        format!(
                            "charged_cell_stack.len()({})>=N_CELLS({})",
                            vec.len(),
                            N_CELLS
                        )
                    );
                    //LOG
                }
            }
            Err(err) => {
                //LOG
                create_internal_log_msg!(
                    planet_id,
                    ERR_LOG_CHNL,
                    "Action".to_string(),
                    "push_charged_cell".to_string(),
                    "ERR".to_string(),
                    format!("{:?}", err)
                );
                //LOG
            }
        }
    }

    // TODO: this is a legacy function.
    // TODO:  we should remove this

    /// checks wether there is an available charged cell,
    /// without actually consuming the value.
    /// Returns Some and the corresponding index or
    /// None if there are no charged cells.
    #[allow(dead_code)]
    pub fn peek_charged_cell_index(planet_id: u64) -> Option<u32> {
        let charged_cell_stack = CHARGED_CELL_STACK.lock();
        //LOG
        create_internal_log_msg!(
            planet_id,
            TRACE_LOG_CHNL,
            "Action".to_string(),
            "CHARGED_CELL_STACK.lock()".to_string()
        );
        //LOG
        let res;
        match charged_cell_stack {
            Ok(vec) => {
                res = vec.last().copied();
                //LOG
                create_internal_log_msg!(
                    planet_id,
                    TRACE_LOG_CHNL,
                    "Action".to_string(),
                    "charged_cell_stack.last().copied()".to_string()
                );
                //LOG
            }
            Err(err) => {
                //LOG
                create_internal_log_msg!(
                    planet_id,
                    ERR_LOG_CHNL,
                    "Action".to_string(),
                    "peek_charged_cell_index".to_string(),
                    "ERR".to_string(),
                    format!("{:?}", err)
                );
                //LOG
                res = None;
            }
        }
        //LOG
        create_internal_log_msg!(
            planet_id,
            DEBUG_LOG_CHNL,
            "Action".to_string(),
            "peek_charged_cell_index".to_string(),
            "Result".to_string(),
            format!("{:?}", res)
        );
        //LOG
        res
    }
}
