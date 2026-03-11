import { NgModule } from '@angular/core';
import { CommonModule } from '@angular/common';
import { FormsModule, ReactiveFormsModule } from '@angular/forms';
import {
    NbCardModule,
    NbButtonModule,
    NbInputModule,
    NbSelectModule,
    NbIconModule,
    NbSpinnerModule,
    NbCheckboxModule,
    NbStepperModule,
    NbListModule,
    NbUserModule,
    NbTreeGridModule,
    NbTooltipModule,
    NbTabsetModule,
    NbTagModule
} from '@nebular/theme';
import { DataSyncWrapperComponent } from './data-sync.component';
import { SyncRequestComponent } from './sync-request/sync-request.component';
import { SyncApprovalComponent } from './sync-approval/sync-approval.component';
import { RouterModule, Routes } from '@angular/router';

const routes: Routes = [
    {
        path: '',
        component: SyncRequestComponent,
    },
];

@NgModule({
    declarations: [
        DataSyncWrapperComponent,
        SyncRequestComponent,
        SyncApprovalComponent
    ],
    imports: [
        CommonModule,
        FormsModule,
        ReactiveFormsModule,
        NbCardModule,
        NbButtonModule,
        NbInputModule,
        NbSelectModule,
        NbIconModule,
        NbSpinnerModule,
        NbCheckboxModule,
        NbStepperModule,
        NbListModule,
        NbUserModule,
        NbTreeGridModule,
        NbTooltipModule,
        NbTabsetModule,
        NbTagModule,
        RouterModule.forChild(routes),
    ],
})
export class DataSyncModule { }
