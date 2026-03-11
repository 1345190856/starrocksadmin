import { NgModule } from '@angular/core';
import { CommonModule } from '@angular/common';
import { FormsModule } from '@angular/forms';
import {
    NbCardModule,
    NbInputModule,
    NbButtonModule,
    NbIconModule,
    NbSpinnerModule,
    NbCheckboxModule,
    NbSelectModule,
    NbTooltipModule,
} from '@nebular/theme';

import { HeadcountRoutingModule } from './headcount-routing.module';
import { HeadcountListComponent } from './headcount-list/headcount-list.component';

@NgModule({
    declarations: [
        HeadcountListComponent,
    ],
    imports: [
        CommonModule,
        FormsModule,
        NbCardModule,
        NbInputModule,
        NbButtonModule,
        NbIconModule,
        NbSpinnerModule,
        NbCheckboxModule,
        NbSelectModule,
        NbTooltipModule,
        HeadcountRoutingModule,
    ],
})
export class HeadcountModule { }
