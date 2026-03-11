
import { NgModule } from '@angular/core';
import { CommonModule } from '@angular/common';
import { FormsModule } from '@angular/forms';
import {
    NbCardModule,
    NbLayoutModule,
    NbSelectModule,
    NbDatepickerModule,
    NbIconModule,
    NbButtonModule,
    NbInputModule,
    NbAccordionModule,
    NbSpinnerModule,
} from '@nebular/theme';
import { ResourceModule } from '../pages/resource/resource.module';
import { ShareRoutingModule } from './share-routing.module';
import { DepartmentResourceComponent } from './resource/department-resource.component';
import { ResourceService } from '../@core/services/resource.service';
import { SharedResourceService } from './resource/shared-resource.service';

@NgModule({
    imports: [
        CommonModule,
        FormsModule,
        NbLayoutModule,
        NbCardModule,
        NbSelectModule,
        NbDatepickerModule,
        NbIconModule,
        NbButtonModule,
        NbInputModule,
        NbAccordionModule,
        NbSpinnerModule,
        ResourceModule, // Imports ChartPanelComponent
        ShareRoutingModule,
    ],
    declarations: [
        DepartmentResourceComponent,
    ],
    providers: [
        { provide: ResourceService, useClass: SharedResourceService }
    ]
})
export class ShareModule { }
