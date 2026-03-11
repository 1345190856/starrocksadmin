import { NgModule } from '@angular/core';
import { CommonModule } from '@angular/common';
import { FormsModule, ReactiveFormsModule } from '@angular/forms';
import { DragDropModule } from '@angular/cdk/drag-drop';

import {
    NbCardModule,
    NbButtonModule,
    NbInputModule,
    NbSelectModule,
    NbCheckboxModule,
    NbSpinnerModule,
    NbAlertModule,
    NbIconModule,
    NbDialogModule,
    NbTooltipModule,
    NbDatepickerModule,
    NbCalendarModule,
    NbAutocompleteModule,
    NbTagModule,
    NbToggleModule,
} from '@nebular/theme';

import { Ng2SmartTableModule } from 'ng2-smart-table';
import { ThemeModule } from '../../@theme/theme.module';

import { DutyRoutingModule } from './duty-routing.module';
import { DutyPersonnelComponent } from './personnel/personnel.component';
import { DutyPersonnelDialogComponent } from './personnel/personnel-dialog/personnel-dialog.component';
import { DutyWheelComponent } from './personnel/duty-wheel/duty-wheel.component';
import { DutyBarrageComponent } from './personnel/duty-barrage/duty-barrage.component';
import { NotifyDialogComponent } from './notify-dialog/notify-dialog.component';

@NgModule({
    declarations: [
        DutyPersonnelComponent,
        DutyPersonnelDialogComponent,
        DutyWheelComponent,
        DutyBarrageComponent,
        NotifyDialogComponent,
    ],
    imports: [
        CommonModule,
        FormsModule,
        ReactiveFormsModule,
        DutyRoutingModule,
        ThemeModule,
        NbCardModule,
        NbButtonModule,
        NbInputModule,
        NbSelectModule,
        NbCheckboxModule,
        NbSpinnerModule,
        NbAlertModule,
        NbIconModule,
        NbDialogModule.forChild(),
        NbTooltipModule,
        NbDatepickerModule,
        NbCalendarModule,
        NbAutocompleteModule,
        NbTagModule,
        NbToggleModule,
        DragDropModule,
        Ng2SmartTableModule,
    ],
})
export class DutyModule { }
