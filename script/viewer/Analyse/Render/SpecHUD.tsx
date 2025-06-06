import {PlayersSpec} from './PlayerSpec';
import {KillFeed} from './KillFeed';
import {AsyncParser} from "../Data/AsyncParser";
import {PlayerState, Kill, Event} from "../Data/Parser";

export interface SpecHUDProps {
    tick: number;
    parser: AsyncParser;
    players: PlayerState[];
    events: Event[];
    onHover: (userId: number) => void;
    highlighted: number | null;
}

export function SpecHUD(props: SpecHUDProps) {
    return (<div class="spechud">
        <KillFeed tick={props.tick} events={props.events} players={props.players}/>
        <PlayersSpec players={props.players} onHover={props.onHover} highlighted={props.highlighted}/>
    </div>)
}

