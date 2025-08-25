export type WidgetType = 'chart' | 'table' | 'notification' | 'metric' | 'progress';

export interface Widget {
	id: string;
	type: WidgetType;
	title: string;
	x: number;
	y: number;
	width: number;
	height: number;
	data?: any;
}

export interface DashboardLayout {
	id: string;
	name: string;
	widgets: Widget[];
	gridCols: number;
	gridRows: number;
}
