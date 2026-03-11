import { NgModule } from '@angular/core';
import { CommonModule } from '@angular/common';
import { FormsModule, ReactiveFormsModule } from '@angular/forms';
import {
    NbCardModule,
    NbButtonModule,
    NbInputModule,
    NbSelectModule,
    NbIconModule,
    NbDialogModule,
    NbTooltipModule,
    NbDatepickerModule,
    NbAccordionModule,
    NbTabsetModule,
    NbSpinnerModule,
    NbCheckboxModule,
} from '@nebular/theme';
import { DragDropModule } from '@angular/cdk/drag-drop';
import { NgxEchartsModule } from 'ngx-echarts';

import { ResourceRoutingModule } from './resource-routing.module';
import { ResourceLayoutComponent } from './layout/resource-layout.component';
import { ChartPanelComponent } from './components/chart-panel/chart-panel.component';
import { PanelEditorComponent } from './components/panel-editor/panel-editor.component';
import { DataSourceListComponent } from './components/datasource-list/datasource-list.component';
import { DataSourceDialogComponent } from './components/datasource-dialog/datasource-dialog.component';

@NgModule({
    declarations: [
        ResourceLayoutComponent,
        ChartPanelComponent,
        PanelEditorComponent,
        DataSourceListComponent,
        DataSourceDialogComponent
    ],
    imports: [
        CommonModule,
        FormsModule,
        ReactiveFormsModule,
        ResourceRoutingModule,
        NbCardModule,
        NbButtonModule,
        NbInputModule,
        NbSelectModule,
        NbIconModule,
        NbDialogModule.forChild(),
        NbTooltipModule,
        NbDatepickerModule,
        NbAccordionModule,
        NbTabsetModule,
        NbSpinnerModule,
        NbCheckboxModule,
        DragDropModule,
        NgxEchartsModule.forChild(),
    ],
    exports: [
        ChartPanelComponent
    ]
})
export class ResourceModule { }
