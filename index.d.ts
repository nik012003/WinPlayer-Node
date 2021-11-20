export interface IPlayer {
    constructor(callback: Function): IPlayer;
    getUpdate(): Update|null;
    Play(): void;
    Pause(): void;
    PlayPause(): void;
    Stop(): void;
    Next(): void;
    Previous(): void;
    Shuffle(): void;
    Repeat(): void;
    Seek(offset: number): void;
    SeekPercentage(percentage: number): void;
    GetPosition(): number;
    SetPosition(position: number): void;
    // @deprecated
    GetVolume(): number;
    // @deprecated
    SetVolume(volume: float): number;
};

export type ArtData = {
	data: Buffer;
	size: number;
	type: string;
};

export type Metadata = {
	id: string;
    title: string;
	artist: string;
	artists: string[];
	album: string;
	albumArtist: string;
	albumArtists: string[];
	artData: ArtData;
	length: number;
};

export type Capabilities = {
	canControl: boolean;
	canPlayPause: boolean;
	canGoNext: boolean;
	canGoPrevious: boolean;
	canSeek: boolean;
};

export type Update = {
	metadata?: Metadata;
	capabilities: Capabilities;
	status: string;
	loop: string;
	shuffle: boolean;
	volume: number;
	elapsed: number;
	app: string;
	appName: string;
};

declare const Player: IPlayer;
export default Player;
